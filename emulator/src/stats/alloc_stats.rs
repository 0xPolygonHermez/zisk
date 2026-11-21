//! Allocator precompile estimation (`--alloc-stats`).
//!
//! Estimates what the allocator precompile (`precompiles/allocator`) would cost for a program
//! that today runs its own allocator inside the guest, without changing the guest:
//!
//! 1. **Capture.** The calls to the allocation entry points of the guest (`__rust_alloc` & co,
//!    `malloc`/`free`/..., `operator new`/`delete`) are detected by their ELF symbols. Arguments
//!    and result are read from the registers, and the cost the guest spends inside each call is
//!    measured (inclusive: everything between the entry and the return).
//! 2. **Replay.** The same sequence of allocations is replayed on shadow heaps that follow the
//!    rules of the precompile, to count the rows each variant would need:
//!    - `Allocator`: best fit and forced coalescing, one row per operation.
//!    - `AllocatorFlex`, eager joins: same heap, but every join is a row of its own.
//!    - `AllocatorFlex`, lazy joins: free blocks are only joined when no block fits.
//! 3. **Report.** Guest cost against the estimated cost of each variant, plus the distributions
//!    that drive the open design decisions: sizes, alignments and `realloc`.
//!
//! The shadow heaps have their own addresses: only the sequence of sizes is taken from the guest.
//!
//! What the estimation assumes:
//! - Sizes are rounded up to 8 bytes, a zero size counts as 8.
//! - An alignment bigger than 8 is served by over-allocating `align - 8` bytes.
//! - `realloc` is three instructions: `alloc`, `memcpy` and `dealloc`.
//! - Each precompile operation costs one main step. The steps of the wrapper around it (rounding
//!   the size, the alignment fix-up) are not counted.
//! - A zeroed allocation only pays a `memset` when the block it gets has been used before.

use std::collections::{BTreeMap, HashMap};

use zisk_core::{
    AllocHeap, AllocInstContext, AllocPolicy, ALLOC_FLEX_ROW_COST, ALLOC_ROW_COST,
    DMA_64_ALIGNED_MEMCPY_COST, DMA_64_ALIGNED_MEMCPY_DIVISOR, DMA_64_ALIGNED_MEMSET_COST,
    DMA_64_ALIGNED_MEMSET_DIVISOR, DMA_MEMCPY_COST, DMA_MEMSET_COST, MAIN_COST,
};

use crate::{StatsReport, MEM_READ_COST, MEM_WRITE_COST};

/// Size of the shadow heap when the ELF does not tell the size of the guest heap.
const DEFAULT_HEAP_SIZE: u64 = 1 << 31;

const REG_RA: usize = 1;
const REG_SP: usize = 2;
const REG_A0: usize = 10;

/// Signature of an allocation entry point, which tells how to read its arguments.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocEntryKind {
    /// `(size, align) -> ptr`
    RustAlloc,
    /// `(size, align) -> ptr`, zeroed
    RustAllocZeroed,
    /// `(ptr, size, align)`
    RustDealloc,
    /// `(ptr, old_size, align, new_size) -> ptr`
    RustRealloc,
    /// `(size) -> ptr`
    Malloc,
    /// `(count, size) -> ptr`, zeroed
    Calloc,
    /// `(ptr)`
    Free,
    /// `(ptr, new_size) -> ptr`
    Realloc,
    /// `(align, size) -> ptr`
    AlignedAlloc,
    /// `(size, align) -> ptr`
    NewAligned,
}

impl AllocEntryKind {
    /// Recognizes an allocation entry point by its (demangled) symbol name.
    pub fn from_symbol(name: &str) -> Option<Self> {
        // C++: `operator new(unsigned long)`, `operator delete[](void*, unsigned long)`, ...
        if name.starts_with("operator new") {
            return Some(if name.contains("align_val_t") {
                Self::NewAligned
            } else {
                Self::Malloc
            });
        }
        if name.starts_with("operator delete") {
            return Some(Self::Free);
        }
        // Rust: `__rust_alloc` (legacy mangling) or `__rustc::__rust_alloc` (v0). The `__rg_` and
        // `__rdl_` symbols are the same calls one level down, for binaries where the shim was
        // inlined; when both exist only the outermost call is taken.
        let last = name.rsplit("::").next().unwrap_or(name);
        let rust = last
            .strip_prefix("__rust_")
            .or_else(|| last.strip_prefix("__rg_"))
            .or_else(|| last.strip_prefix("__rdl_"));
        match rust {
            Some("alloc") => return Some(Self::RustAlloc),
            Some("alloc_zeroed") => return Some(Self::RustAllocZeroed),
            Some("dealloc") => return Some(Self::RustDealloc),
            Some("realloc") => return Some(Self::RustRealloc),
            _ => {}
        }
        match name {
            "malloc" => Some(Self::Malloc),
            "calloc" => Some(Self::Calloc),
            "free" => Some(Self::Free),
            "realloc" => Some(Self::Realloc),
            "aligned_alloc" | "memalign" => Some(Self::AlignedAlloc),
            // bump allocator of ziskos, called directly by C code
            "sys_alloc_aligned" => Some(Self::RustAlloc),
            _ => None,
        }
    }

    /// Reads the call from the argument registers `a0..a3`.
    fn decode(self, a: [u64; 4]) -> Call {
        match self {
            Self::RustAlloc | Self::NewAligned => {
                Call::Alloc { size: a[0], align: a[1], zeroed: false }
            }
            Self::RustAllocZeroed => Call::Alloc { size: a[0], align: a[1], zeroed: true },
            Self::RustDealloc | Self::Free => Call::Free { ptr: a[0] },
            Self::RustRealloc => Call::Realloc { ptr: a[0], size: a[3], align: a[2] },
            Self::Malloc => Call::Alloc { size: a[0], align: DEFAULT_ALIGN, zeroed: false },
            Self::Calloc => {
                Call::Alloc { size: a[0].saturating_mul(a[1]), align: DEFAULT_ALIGN, zeroed: true }
            }
            Self::Realloc => Call::Realloc { ptr: a[0], size: a[1], align: DEFAULT_ALIGN },
            Self::AlignedAlloc => Call::Alloc { size: a[1], align: a[0], zeroed: false },
        }
    }
}

/// Alignment recorded for the calls that do not state one (`malloc`, `operator new`).
const DEFAULT_ALIGN: u64 = 0;

#[derive(Clone, Copy, Debug)]
enum Call {
    Alloc { size: u64, align: u64, zeroed: bool },
    Free { ptr: u64 },
    Realloc { ptr: u64, size: u64, align: u64 },
}

/// One of the heaps the allocations are replayed on: the heap of the precompile for one policy,
/// with a single clean area, plus what the replay costs on it.
#[derive(Debug)]
struct ShadowHeap {
    heap: AllocHeap,
    /// Zeroed allocations that got a used block, the words they have to clear and their cost.
    memsets: u64,
    memset_words: u64,
    memset_cost: u64,
    /// Allocations that found no block, with the area exhausted.
    failures: u64,
}

impl ShadowHeap {
    fn new(policy: AllocPolicy, size: u64) -> Self {
        let mut heap = AllocHeap::new(policy);
        heap.create(0, size, false).expect("shadow heap area");
        Self { heap, memsets: 0, memset_words: 0, memset_cost: 0, failures: 0 }
    }

    fn alloc(&mut self, size: u64, zeroed: bool) -> Option<u64> {
        let Ok(outcome) = self.heap.alloc(size) else {
            self.failures += 1;
            return None;
        };
        if zeroed && outcome.dirty {
            self.memsets += 1;
            self.memset_words += size / 8;
            self.memset_cost += memset_cost(size / 8);
        }
        Some(outcome.addr)
    }

    fn dealloc(&mut self, addr: u64) {
        self.heap.dealloc(addr).expect("block of the shadow heap");
    }

    /// Rows of the air, the create of the area included.
    fn rows(&self) -> u64 {
        self.heap.rows()
    }

    /// Operations the main sends along the replay (the create of the area is not one of them).
    fn main_ops(&self) -> u64 {
        self.heap.op_rows - 1
    }
}

/// Cost of a `memset` of `words` aligned 64-bit words launched by the precompile (no main step).
fn memset_cost(words: u64) -> u64 {
    DMA_MEMSET_COST
        + words.div_ceil(DMA_64_ALIGNED_MEMSET_DIVISOR) * DMA_64_ALIGNED_MEMSET_COST
        + words * MEM_WRITE_COST
}

/// Cost of a `memcpy` of `words` aligned 64-bit words executed by the main.
fn memcpy_cost(words: u64) -> u64 {
    MAIN_COST
        + DMA_MEMCPY_COST
        + words.div_ceil(DMA_64_ALIGNED_MEMCPY_DIVISOR) * DMA_64_ALIGNED_MEMCPY_COST
        + words * (MEM_READ_COST + MEM_WRITE_COST)
}

/// Size the precompile is asked for: a non-zero multiple of 8, with room to align inside it.
fn shadow_size(size: u64, align: u64) -> u64 {
    let size = size.max(1).div_ceil(8) * 8;
    if align > 8 {
        size + align - 8
    } else {
        size
    }
}

const HEAPS: usize = AllocPolicy::ALL.len();

/// A block the guest has allocated and not released yet.
#[derive(Debug)]
struct Live {
    /// Size the guest asked for.
    size: u64,
    /// Size asked to the precompile.
    shadow_size: u64,
    /// Address of the block in each shadow heap, `None` if that heap was exhausted.
    addr: [Option<u64>; HEAPS],
}

/// What the guest spends in one of its allocation entry points.
#[derive(Debug)]
struct EntryStats {
    pc: u64,
    name: String,
    kind: AllocEntryKind,
    calls: u64,
    steps: u64,
    cost: u64,
}

/// An entry point that has been entered and has not returned yet.
#[derive(Debug)]
struct Pending {
    entry: usize,
    ra: u64,
    sp: u64,
    call: Call,
    steps: u64,
    cost: u64,
}

#[derive(Default, Debug, Clone, Copy)]
struct Bucket {
    calls: u64,
    bytes: u64,
}

#[derive(Default, Debug)]
struct ReallocStats {
    calls: u64,
    grow: u64,
    shrink: u64,
    /// The size asked to the precompile does not change: nothing to do.
    same: u64,
    /// Growing ones whose block could have grown without moving in the `Allocator` heap.
    grow_in_place: u64,
    copied_words: u64,
    copy_cost: u64,
    /// Words copied by reallocs that could have been served in place (same, shrink, grow in
    /// place), and the cost of those copies.
    avoidable_words: u64,
    avoidable_cost: u64,
}

#[derive(Debug)]
pub struct AllocStats {
    /// Entry points sorted by pc, to find them by binary search.
    entries: Vec<EntryStats>,
    min_pc: u64,
    max_pc: u64,
    pending: Option<Pending>,
    heaps: [ShadowHeap; HEAPS],
    heap_size: u64,
    live: HashMap<u64, Live>,
    live_bytes: u64,
    peak_live_bytes: u64,
    peak_live_blocks: usize,
    allocs: u64,
    zeroed_allocs: u64,
    frees: u64,
    reallocs: ReallocStats,
    /// Calls to an entry point made from inside another one, which are not counted again.
    nested: u64,
    /// Entered calls whose return was never seen.
    lost: u64,
    null_frees: u64,
    unknown_frees: u64,
    failed_allocs: u64,
    /// Allocations that returned an address still in use: a release was not seen.
    reused: u64,
    /// Allocations by size, in power-of-two buckets indexed by the bit length of the size.
    by_size: [Bucket; 65],
    /// Allocations by alignment, `DEFAULT_ALIGN` for those that do not state one.
    by_align: BTreeMap<u64, Bucket>,
}

impl AllocStats {
    /// `symbols` are the functions of the ELF as `(address, demangled name)`; `heap_size` the size
    /// of the guest heap, `0` if unknown.
    pub fn new<'a>(symbols: impl Iterator<Item = (u64, &'a str)>, heap_size: u64) -> Self {
        let mut entries: Vec<EntryStats> = symbols
            .filter_map(|(pc, name)| {
                AllocEntryKind::from_symbol(name).map(|kind| EntryStats {
                    pc,
                    name: name.to_string(),
                    kind,
                    calls: 0,
                    steps: 0,
                    cost: 0,
                })
            })
            .collect();
        entries.sort_by_key(|e| e.pc);
        // Aliases (several symbols at one address) are one entry point.
        entries.dedup_by(|b, a| {
            if a.pc == b.pc {
                a.name = format!("{} | {}", a.name, b.name);
                true
            } else {
                false
            }
        });
        let heap_size = if heap_size == 0 { DEFAULT_HEAP_SIZE } else { heap_size & !7 };
        Self {
            min_pc: entries.first().map_or(u64::MAX, |e| e.pc),
            max_pc: entries.last().map_or(0, |e| e.pc),
            entries,
            pending: None,
            heaps: [
                ShadowHeap::new(AllocPolicy::Canonical, heap_size),
                ShadowHeap::new(AllocPolicy::FlexEager, heap_size),
                ShadowHeap::new(AllocPolicy::FlexLazy, heap_size),
            ],
            heap_size,
            live: HashMap::new(),
            live_bytes: 0,
            peak_live_bytes: 0,
            peak_live_blocks: 0,
            allocs: 0,
            zeroed_allocs: 0,
            frees: 0,
            reallocs: ReallocStats::default(),
            nested: 0,
            lost: 0,
            null_frees: 0,
            unknown_frees: 0,
            failed_allocs: 0,
            reused: 0,
            by_size: [Bucket::default(); 65],
            by_align: BTreeMap::new(),
        }
    }

    /// Names of the entry points found in the ELF.
    pub fn entry_names(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.name.as_str()).collect()
    }

    /// Whether the instruction at `pc` is one `on_step` has to see: an entry point, or the return
    /// address of the call in progress. Cheap, it is evaluated on every step.
    #[inline(always)]
    pub fn is_watched(&self, pc: u64) -> bool {
        (pc >= self.min_pc && pc <= self.max_pc)
            || self.pending.as_ref().is_some_and(|p| p.ra == pc)
    }

    /// Called for the instructions `is_watched` selects, with the registers as they are before
    /// the instruction writes its result, and the steps and cost of the program up to it.
    pub fn on_step(&mut self, pc: u64, regs: &[u64], steps: u64, cost: u64) {
        let sp = regs[REG_SP];
        if let Some(pending) = &self.pending {
            if pending.ra == pc && pending.sp == sp {
                let pending = self.pending.take().unwrap();
                let entry = &mut self.entries[pending.entry];
                entry.calls += 1;
                entry.steps += steps - pending.steps;
                entry.cost += cost - pending.cost;
                self.on_call(pending.call, regs[REG_A0]);
                // the return address could be an entry point itself (a tail position), go on
            }
        }
        let Ok(entry) = self.entries.binary_search_by_key(&pc, |e| e.pc) else {
            return;
        };
        let ra = regs[REG_RA];
        if let Some(pending) = &self.pending {
            // Inside another entry point the stack is deeper, or it is the same frame when the
            // outer one jumped here (a tail call keeps sp and ra).
            if sp < pending.sp || (sp == pending.sp && ra == pending.ra) {
                self.nested += 1;
                return;
            }
            // Otherwise the outer call ended without being seen (it never returned to its caller).
            self.lost += 1;
        }
        let a = [regs[REG_A0], regs[REG_A0 + 1], regs[REG_A0 + 2], regs[REG_A0 + 3]];
        self.pending =
            Some(Pending { entry, ra, sp, call: self.entries[entry].kind.decode(a), steps, cost });
    }

    fn on_call(&mut self, call: Call, result: u64) {
        match call {
            Call::Alloc { size, align, zeroed } => self.on_alloc(size, align, zeroed, result),
            Call::Free { ptr } => self.on_free(ptr),
            Call::Realloc { ptr, size, align } => self.on_realloc(ptr, size, align, result),
        }
    }

    fn on_alloc(&mut self, size: u64, align: u64, zeroed: bool, ptr: u64) {
        if ptr == 0 {
            self.failed_allocs += 1;
            return;
        }
        self.allocs += 1;
        self.zeroed_allocs += zeroed as u64;
        self.count_alloc(size, align);
        self.shadow_alloc(ptr, size, align, zeroed);
    }

    fn count_alloc(&mut self, size: u64, align: u64) {
        let bucket = &mut self.by_size[(64 - size.leading_zeros()) as usize];
        bucket.calls += 1;
        bucket.bytes += size;
        let bucket = self.by_align.entry(align).or_default();
        bucket.calls += 1;
        bucket.bytes += size;
    }

    fn shadow_alloc(&mut self, ptr: u64, size: u64, align: u64, zeroed: bool) {
        if self.live.contains_key(&ptr) {
            self.reused += 1;
            self.shadow_free(ptr);
        }
        let shadow_size = shadow_size(size, align);
        let mut addr = [None; HEAPS];
        for (heap, addr) in self.heaps.iter_mut().zip(addr.iter_mut()) {
            *addr = heap.alloc(shadow_size, zeroed);
        }
        self.live.insert(ptr, Live { size, shadow_size, addr });
        self.live_bytes += size;
        self.peak_live_bytes = self.peak_live_bytes.max(self.live_bytes);
        self.peak_live_blocks = self.peak_live_blocks.max(self.live.len());
    }

    fn shadow_free(&mut self, ptr: u64) -> bool {
        let Some(live) = self.live.remove(&ptr) else {
            return false;
        };
        self.live_bytes -= live.size;
        for (heap, addr) in self.heaps.iter_mut().zip(live.addr) {
            if let Some(addr) = addr {
                heap.dealloc(addr);
            }
        }
        true
    }

    fn on_free(&mut self, ptr: u64) {
        if ptr == 0 {
            self.null_frees += 1;
        } else if self.shadow_free(ptr) {
            self.frees += 1;
        } else {
            self.unknown_frees += 1;
        }
    }

    fn on_realloc(&mut self, ptr: u64, size: u64, align: u64, new_ptr: u64) {
        let Some(old) = self.live.get(&ptr) else {
            // realloc(NULL, size) is a malloc; an unknown block can only be counted as one
            if ptr != 0 {
                self.unknown_frees += 1;
            }
            return self.on_alloc(size, align, false, new_ptr);
        };
        if new_ptr == 0 {
            // C: realloc(ptr, 0) releases the block; otherwise it failed and the block stays
            if size == 0 {
                self.on_free(ptr);
            } else {
                self.failed_allocs += 1;
            }
            return;
        }
        let new_shadow_size = shadow_size(size, align);
        let words = old.shadow_size.min(new_shadow_size) / 8;
        let copy_cost = memcpy_cost(words);
        let stats = &mut self.reallocs;
        stats.calls += 1;
        stats.copied_words += words;
        stats.copy_cost += copy_cost;
        let in_place = if new_shadow_size == old.shadow_size {
            stats.same += 1;
            true
        } else if new_shadow_size < old.shadow_size {
            stats.shrink += 1;
            true
        } else {
            stats.grow += 1;
            let grows =
                old.addr[0].is_some_and(|addr| self.heaps[0].heap.can_grow(addr, new_shadow_size));
            stats.grow_in_place += grows as u64;
            grows
        };
        if in_place {
            stats.avoidable_words += words;
            stats.avoidable_cost += copy_cost;
        }
        // alloc + memcpy + dealloc. The guest may have returned the same address (it grew in
        // place), so the old block is taken out of the map before the new one goes in.
        let old = self.live.remove(&ptr).unwrap();
        self.live_bytes -= old.size;
        self.count_alloc(size, align);
        self.shadow_alloc(new_ptr, size, align, false);
        for (heap, addr) in self.heaps.iter_mut().zip(old.addr) {
            if let Some(addr) = addr {
                heap.dealloc(addr);
            }
        }
    }

    /// Estimated cost of the variant `heap`: rows, main steps and memsets, plus the copies of the
    /// reallocs.
    fn estimated_cost(&self, heap: usize) -> u64 {
        let h = &self.heaps[heap];
        h.rows() * h.heap.policy().row_cost()
            + h.main_ops() * MAIN_COST
            + h.memset_cost
            + self.reallocs.copy_cost
    }

    /// `total_cost` is the variable cost of the whole program (the base cost is not included), the
    /// reference of the `% VAR` columns.
    pub fn report(&self, report: &mut StatsReport, total_cost: u64) {
        let n = |v: u64| report.format_number(v);
        let mut out = String::new();
        out += "\nALLOCATOR PRECOMPILE ESTIMATION\n";
        out += "-------------------------------\n";
        if self.entries.is_empty() {
            out += "No allocation entry point found in the ELF symbols.\n";
            report.add(&out);
            return;
        }

        // What the guest does today
        let guest_cost: u64 = self.entries.iter().map(|e| e.cost).sum();
        let guest_steps: u64 = self.entries.iter().map(|e| e.steps).sum();
        let guest_calls: u64 = self.entries.iter().map(|e| e.calls).sum();
        out += "\nGUEST ALLOCATOR (cost inside each entry point, callees included)\n";
        out += &format!(
            "{:<44} {:>12} {:>14} {:>16} {:>9} {:>8}\n",
            "ENTRY POINT", "CALLS", "STEPS", "COST", "COST/CALL", "% VAR"
        );
        for e in &self.entries {
            out += &format!(
                "{:<44} {:>12} {:>14} {:>16} {:>9} {:>8}\n",
                clip(&e.name, 44),
                n(e.calls),
                n(e.steps),
                n(e.cost),
                per(e.cost, e.calls),
                perc(e.cost, total_cost),
            );
        }
        out += &format!(
            "{:<44} {:>12} {:>14} {:>16} {:>9} {:>8}\n",
            "TOTAL",
            n(guest_calls),
            n(guest_steps),
            n(guest_cost),
            per(guest_cost, guest_calls),
            perc(guest_cost, total_cost),
        );

        // The sequence that was replayed
        out += "\nOPERATIONS\n";
        out += &format!(
            "allocs {} ({} zeroed)   frees {}   reallocs {}\n",
            n(self.allocs),
            n(self.zeroed_allocs),
            n(self.frees),
            n(self.reallocs.calls)
        );
        out += &format!(
            "peak live: {} bytes in {} blocks   still live at the end: {} bytes in {} blocks\n",
            n(self.peak_live_bytes),
            n(self.peak_live_blocks as u64),
            n(self.live_bytes),
            n(self.live.len() as u64)
        );
        let ignored = [
            ("free(NULL)", self.null_frees),
            ("free of an unknown block", self.unknown_frees),
            ("failed alloc", self.failed_allocs),
            ("address reused without a free", self.reused),
            ("nested call", self.nested),
            ("call that never returned", self.lost),
        ];
        let ignored: Vec<String> = ignored
            .iter()
            .filter(|(_, count)| *count > 0)
            .map(|(name, count)| format!("{name} {}", n(*count)))
            .collect();
        if !ignored.is_empty() {
            out += &format!("not replayed: {}\n", ignored.join(", "));
        }

        // Estimation
        out += &format!(
            "\nPRECOMPILE ESTIMATION (shadow heap of {} bytes; row cost {} / {})\n",
            n(self.heap_size),
            ALLOC_ROW_COST,
            ALLOC_FLEX_ROW_COST
        );
        out += &format!(
            "{:<20} {:>12} {:>12} {:>10} {:>14} {:>14} {:>14} {:>9} {:>8} {:>8}\n",
            "VARIANT",
            "ROWS",
            "JOIN ROWS",
            "MEMSETS",
            "MEMSET COST",
            "HIGH WATER",
            "COST",
            "COST/OP",
            "% VAR",
            "vs GUEST"
        );
        for (i, heap) in self.heaps.iter().enumerate() {
            let cost = self.estimated_cost(i);
            out += &format!(
                "{:<20} {:>12} {:>12} {:>10} {:>14} {:>14} {:>14} {:>9} {:>8} {:>8}\n",
                heap.heap.policy().name(),
                n(heap.rows()),
                n(heap.heap.join_rows),
                n(heap.memsets),
                n(heap.memset_cost),
                n(heap.heap.high_water),
                n(cost),
                per(cost, guest_calls),
                perc(cost, total_cost),
                if guest_cost == 0 {
                    "-".to_string()
                } else {
                    format!("{:.2}x", cost as f64 / guest_cost as f64)
                },
            );
            if heap.failures > 0 {
                out += &format!(
                    "  WARNING: {} allocations did not fit in the shadow heap\n",
                    n(heap.failures)
                );
            }
        }
        out += &format!(
            "COST = rows x row cost + operations x {MAIN_COST} (one main step each) + memsets + \
             realloc copies ({}).\n",
            n(self.reallocs.copy_cost)
        );

        // realloc
        let r = &self.reallocs;
        if r.calls > 0 {
            out += "\nREALLOC (replayed as alloc + memcpy + dealloc)\n";
            out += &format!(
                "calls {}: grow {} ({} could grow in place), shrink {}, same size {}\n",
                n(r.calls),
                n(r.grow),
                n(r.grow_in_place),
                n(r.shrink),
                n(r.same)
            );
            out += &format!(
                "copied {} bytes, cost {} ({} per call)\n",
                n(r.copied_words * 8),
                n(r.copy_cost),
                per(r.copy_cost, r.calls)
            );
            // In place there is no copy, and no second row: one operation instead of two.
            let in_place = r.same + r.shrink + r.grow_in_place;
            out += &format!(
                "an in-place realloc would avoid {} copies ({}), {} bytes, cost {} ({})\n",
                n(in_place),
                perc(in_place, r.calls),
                n(r.avoidable_words * 8),
                n(r.avoidable_cost),
                perc(r.avoidable_cost, r.copy_cost)
            );
        }

        // Alignments
        out += "\nALIGNMENT OF THE ALLOCATIONS (reallocs included)\n";
        out += &format!("{:<28} {:>12} {:>9} {:>16} {:>9}\n", "ALIGN", "CALLS", "%", "BYTES", "%");
        let total: Bucket = self.by_align.values().fold(Bucket::default(), |mut acc, b| {
            acc.calls += b.calls;
            acc.bytes += b.bytes;
            acc
        });
        for (&align, bucket) in &self.by_align {
            let label = if align == DEFAULT_ALIGN {
                "not stated (malloc/new)".to_string()
            } else {
                align.to_string()
            };
            out += &format!(
                "{:<28} {:>12} {:>9} {:>16} {:>9}\n",
                label,
                n(bucket.calls),
                perc(bucket.calls, total.calls),
                n(bucket.bytes),
                perc(bucket.bytes, total.bytes)
            );
        }
        let over: Bucket = self.by_align.iter().filter(|(&align, _)| align > 8).fold(
            Bucket::default(),
            |mut acc, (_, b)| {
                acc.calls += b.calls;
                acc.bytes += b.bytes;
                acc
            },
        );
        out += &format!(
            "bigger than 8: {} calls ({}), {} bytes ({})\n",
            n(over.calls),
            perc(over.calls, total.calls),
            n(over.bytes),
            perc(over.bytes, total.bytes)
        );

        // Sizes
        out += "\nSIZE OF THE ALLOCATIONS (reallocs included)\n";
        out += &format!("{:<28} {:>12} {:>9} {:>16} {:>9}\n", "BYTES", "CALLS", "%", "BYTES", "%");
        for (bits, bucket) in self.by_size.iter().enumerate() {
            if bucket.calls == 0 {
                continue;
            }
            let label = match bits {
                0 => "0".to_string(),
                1 => "1".to_string(),
                _ => format!("{}-{}", 1u128 << (bits - 1), (1u128 << bits) - 1),
            };
            out += &format!(
                "{:<28} {:>12} {:>9} {:>16} {:>9}\n",
                label,
                n(bucket.calls),
                perc(bucket.calls, total.calls),
                n(bucket.bytes),
                perc(bucket.bytes, total.bytes)
            );
        }
        report.add(&out);
    }
}

/// What the allocator precompile did in an execution that uses it (a guest built with the
/// precompile allocator), taken from the emulation context when the execution ends. Its cost is
/// already part of the totals, under the `alloc_*` opcodes.
#[derive(Debug)]
pub struct AllocPrecompileSummary {
    policy: AllocPolicy,
    op_rows: u64,
    join_rows: u64,
    callocs: u64,
    memsets: u64,
    memset_words: u64,
    used_bytes: u64,
    used_blocks: u64,
    peak_used_bytes: u64,
    peak_used_blocks: u64,
    high_water: u64,
}

impl AllocPrecompileSummary {
    /// `None` when the program never called the precompile.
    pub fn new(alloc: &AllocInstContext) -> Option<Self> {
        let heap = &alloc.heap;
        (heap.op_rows > 0).then(|| Self {
            policy: heap.policy(),
            op_rows: heap.op_rows,
            join_rows: heap.join_rows,
            callocs: alloc.callocs,
            memsets: alloc.memsets,
            memset_words: alloc.memset_words,
            used_bytes: heap.used_bytes,
            used_blocks: heap.used_blocks() as u64,
            peak_used_bytes: heap.peak_used_bytes,
            peak_used_blocks: heap.peak_used_blocks as u64,
            high_water: heap.high_water,
        })
    }

    pub fn report(&self, report: &mut StatsReport) {
        let n = |v: u64| report.format_number(v);
        let rows = self.op_rows + self.join_rows;
        let mut out = String::new();
        out += "\nALLOCATOR PRECOMPILE\n";
        out += "--------------------\n";
        out += &format!(
            "variant {} (row cost {}): {} rows = {} operations + {} joins, cost of the rows {}\n",
            self.policy.name(),
            self.policy.row_cost(),
            n(rows),
            n(self.op_rows),
            n(self.join_rows),
            n(rows * self.policy.row_cost())
        );
        out += &format!(
            "zeroed allocations {}, {} of them over a used block: {} bytes cleared\n",
            n(self.callocs),
            n(self.memsets),
            n(self.memset_words * 8)
        );
        out += &format!(
            "peak in use: {} bytes in {} blocks   still in use at the end: {} bytes in {} blocks\n",
            n(self.peak_used_bytes),
            n(self.peak_used_blocks),
            n(self.used_bytes),
            n(self.used_blocks)
        );
        out += &format!("highest address handed out: 0x{:x}\n", self.high_water);
        report.add(&out);
    }
}

fn perc(value: u64, total: u64) -> String {
    if total == 0 {
        "-".to_string()
    } else {
        format!("{:.2}%", 100.0 * value as f64 / total as f64)
    }
}

fn per(value: u64, count: u64) -> String {
    if count == 0 {
        "-".to_string()
    } else {
        format!("{:.1}", value as f64 / count as f64)
    }
}

fn clip(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        text.to_string()
    } else {
        let mut clipped: String = text.chars().take(width - 1).collect();
        clipped.push('…');
        clipped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MALLOC: u64 = 0x1000;
    const FREE: u64 = 0x2000;
    const REALLOC: u64 = 0x3000;
    const RUST_ALLOC: u64 = 0x4000;
    const RG_ALLOC: u64 = 0x5000;
    const CALLER: u64 = 0x9000;
    const SP: u64 = 0x8000_0000;

    fn stats() -> AllocStats {
        let symbols = [
            (MALLOC, "malloc"),
            (FREE, "free"),
            (REALLOC, "realloc"),
            (RUST_ALLOC, "__rustc::__rust_alloc_zeroed"),
            (RG_ALLOC, "__rg_alloc_zeroed"),
            (0x6000, "main"),
        ];
        AllocStats::new(symbols.into_iter(), 1 << 20)
    }

    fn regs(ra: u64, sp: u64, a: [u64; 4]) -> [u64; 32] {
        let mut regs = [0u64; 32];
        regs[REG_RA] = ra;
        regs[REG_SP] = sp;
        regs[REG_A0..REG_A0 + 4].copy_from_slice(&a);
        regs
    }

    /// Runs a whole call: the entry with its arguments, then the return with `result` in a0.
    fn call(stats: &mut AllocStats, entry: u64, a: [u64; 4], result: u64) {
        assert!(stats.is_watched(entry));
        stats.on_step(entry, &regs(CALLER, SP, a), 100, 1000);
        assert!(stats.is_watched(CALLER));
        stats.on_step(CALLER, &regs(CALLER, SP, [result, 0, 0, 0]), 110, 1500);
    }

    #[test]
    fn symbols_are_classified_by_name() {
        use AllocEntryKind::*;
        for (name, kind) in [
            ("__rust_alloc", Some(RustAlloc)),
            ("__rustc::__rust_dealloc", Some(RustDealloc)),
            ("__rustc::__rust_realloc", Some(RustRealloc)),
            ("__rdl_alloc_zeroed", Some(RustAllocZeroed)),
            ("operator new(unsigned long)", Some(Malloc)),
            ("operator new[](unsigned long, std::nothrow_t const&)", Some(Malloc)),
            ("operator new(unsigned long, std::align_val_t)", Some(NewAligned)),
            ("operator delete(void*, unsigned long, std::align_val_t)", Some(Free)),
            ("calloc", Some(Calloc)),
            ("alloc::raw_vec::finish_grow", None),
            ("__rust_alloc_error_handler", None),
            ("my_malloc", None),
        ] {
            assert_eq!(AllocEntryKind::from_symbol(name), kind, "{name}");
        }
    }

    #[test]
    fn a_call_is_counted_at_its_return_with_the_cost_spent_inside() {
        let mut stats = stats();
        assert!(!stats.is_watched(0x100));
        call(&mut stats, MALLOC, [24, 0, 0, 0], 0xA000);
        assert_eq!((stats.allocs, stats.frees), (1, 0));
        let malloc = stats.entries.iter().find(|e| e.name == "malloc").unwrap();
        assert_eq!((malloc.calls, malloc.steps, malloc.cost), (1, 10, 500));
        call(&mut stats, FREE, [0xA000, 0, 0, 0], 0);
        assert_eq!((stats.allocs, stats.frees, stats.live.len()), (1, 1, 0));
        // create + alloc + dealloc; the released block is joined back with the rest of the area
        assert_eq!(stats.heaps[0].rows(), 3);
        assert_eq!(stats.heaps[0].heap.free_blocks(), 1);
        assert_eq!(stats.heaps[1].heap.join_rows, 1);
        assert_eq!(stats.heaps[2].heap.join_rows, 0);
        assert_eq!(stats.heaps[2].heap.free_blocks(), 2);
    }

    #[test]
    fn a_nested_entry_point_is_not_counted_twice() {
        let mut stats = stats();
        // __rust_alloc_zeroed jumps to __rg_alloc_zeroed (same frame), which calls malloc
        stats.on_step(RUST_ALLOC, &regs(CALLER, SP, [64, 8, 0, 0]), 0, 0);
        stats.on_step(RG_ALLOC, &regs(CALLER, SP, [64, 8, 0, 0]), 1, 10);
        stats.on_step(MALLOC, &regs(RG_ALLOC + 8, SP - 16, [64, 0, 0, 0]), 2, 20);
        stats.on_step(CALLER, &regs(CALLER, SP, [0xB000, 0, 0, 0]), 9, 90);
        assert_eq!((stats.allocs, stats.zeroed_allocs, stats.nested), (1, 1, 2));
        assert_eq!(stats.by_align.get(&8).map(|b| b.calls), Some(1));
    }

    #[test]
    fn a_call_that_never_returns_does_not_hide_the_next_ones() {
        let mut stats = stats();
        stats.on_step(MALLOC, &regs(CALLER, SP - 64, [8, 0, 0, 0]), 0, 0);
        // the stack was unwound past that frame and malloc is called again
        call(&mut stats, MALLOC, [8, 0, 0, 0], 0xA000);
        assert_eq!((stats.allocs, stats.lost), (1, 1));
    }

    #[test]
    fn a_zeroed_alloc_only_pays_a_memset_on_a_used_block() {
        let mut heap = ShadowHeap::new(AllocPolicy::Canonical, 1 << 20);
        let a = heap.alloc(64, true).unwrap();
        let _b = heap.alloc(64, false).unwrap();
        assert_eq!(heap.memsets, 0);
        heap.dealloc(a);
        // best fit takes the released block, which has been used
        assert_eq!(heap.alloc(64, true), Some(a));
        assert_eq!((heap.memsets, heap.memset_words), (1, 8));
        assert_eq!(heap.memset_cost, memset_cost(8));
        // create + 3 allocs + dealloc, and the create is not an operation of the main
        assert_eq!((heap.rows(), heap.main_ops()), (5, 4));
    }

    #[test]
    fn realloc_is_an_alloc_a_copy_and_a_dealloc() {
        let mut stats = stats();
        call(&mut stats, MALLOC, [16, 0, 0, 0], 0xA000);
        // nothing above the block: it could grow in place
        call(&mut stats, REALLOC, [0xA000, 40, 0, 0], 0xB000);
        assert_eq!(
            (stats.reallocs.calls, stats.reallocs.grow, stats.reallocs.grow_in_place),
            (1, 1, 1)
        );
        assert_eq!(stats.reallocs.copied_words, 2);
        assert_eq!(stats.reallocs.copy_cost, memcpy_cost(2));
        assert_eq!(stats.live.get(&0xB000).map(|l| l.shadow_size), Some(40));
        assert!(!stats.live.contains_key(&0xA000));
        // create + alloc + (alloc + dealloc)
        assert_eq!(stats.heaps[0].rows(), 4);
        // the hole it left is filled and a block lands right above it: now it has to move
        call(&mut stats, MALLOC, [16, 0, 0, 0], 0xA000);
        call(&mut stats, MALLOC, [8, 0, 0, 0], 0xC000);
        call(&mut stats, REALLOC, [0xB000, 400, 0, 0], 0xB000);
        assert_eq!((stats.reallocs.grow, stats.reallocs.grow_in_place), (2, 1));
        assert_eq!(stats.live_bytes, 424);
        // shrinking never needs to move
        call(&mut stats, REALLOC, [0xB000, 100, 0, 0], 0xB000);
        assert_eq!((stats.reallocs.shrink, stats.reallocs.same), (1, 0));
        call(&mut stats, REALLOC, [0xB000, 97, 0, 0], 0xB000);
        assert_eq!(stats.reallocs.same, 1);
    }

    #[test]
    fn an_alignment_over_8_is_served_by_over_allocating() {
        assert_eq!(shadow_size(0, 0), 8);
        assert_eq!(shadow_size(13, 8), 16);
        assert_eq!(shadow_size(64, 64), 64 + 56);
        assert_eq!(shadow_size(1, 4096), 8 + 4088);
    }
}
