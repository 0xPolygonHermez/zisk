//! Analysis of how a program uses its allocator (`--alloc-stats`).
//!
//! The calls to the allocator are captured from one of two sources:
//!
//! - **Allocation marks** (`SYSCALL_ALLOC_MARK_ID`, see `zisk_definitions::alloc_mark`): no-op
//!   instructions the guest allocator emits around every call. This is what a Rust guest built
//!   with the `alloc-marks` feature of ziskos does, and the only source that works when the
//!   allocator is inlined at its call sites.
//! - **ELF symbols**: when the program has no marks, the calls to the well-known entry points
//!   (`malloc`/`free`/..., `operator new`/`delete`, `__rust_alloc` & co) are detected by their
//!   address, and arguments and result are read from the registers. This covers C and C++ guests
//!   without touching them.
//!
//! As soon as a mark is seen the symbols are no longer watched, so a call is never counted twice.
//!
//! The report tells what the program asks its allocator for (sizes, alignments, zeroed
//! allocations, reallocations growing and shrinking, how much memory is live) and what the
//! allocator costs today. `--alloc-trace` additionally writes the sequence of calls to a file, to
//! replay it on other allocator designs.

use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufWriter, Write},
};

use zisk_definitions::{
    ALLOC_MARK_ALLOC_ID, ALLOC_MARK_ALLOC_ZEROED_ID, ALLOC_MARK_DEALLOC_ID, ALLOC_MARK_END_ID,
    ALLOC_MARK_LAYOUT_ID, ALLOC_MARK_REALLOC_ID,
};

use crate::{StatsReport, MAIN_COST};

const REG_RA: usize = 1;
const REG_SP: usize = 2;
const REG_A0: usize = 10;

/// Alignment recorded for the calls that do not state one (`malloc`, `operator new`).
const ALIGN_NOT_STATED: u64 = 0;

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
        let none = ALIGN_NOT_STATED;
        match self {
            Self::RustAlloc | Self::NewAligned => {
                Call::Alloc { size: a[0], align: a[1], zeroed: false }
            }
            Self::RustAllocZeroed => Call::Alloc { size: a[0], align: a[1], zeroed: true },
            Self::RustDealloc => Call::Free { ptr: a[0], layout: Some((a[1], a[2])) },
            Self::Free => Call::Free { ptr: a[0], layout: None },
            Self::RustRealloc => {
                Call::Realloc { ptr: a[0], layout: Some((a[1], a[2])), new_size: a[3] }
            }
            Self::Malloc => Call::Alloc { size: a[0], align: none, zeroed: false },
            Self::Calloc => {
                Call::Alloc { size: a[0].saturating_mul(a[1]), align: none, zeroed: true }
            }
            Self::Realloc => Call::Realloc { ptr: a[0], layout: None, new_size: a[1] },
            Self::AlignedAlloc => Call::Alloc { size: a[1], align: a[0], zeroed: false },
        }
    }
}

/// A call to the allocator. `layout` is the `(size, align)` the caller states for the block it
/// passes, when it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Call {
    Alloc { size: u64, align: u64, zeroed: bool },
    Free { ptr: u64, layout: Option<(u64, u64)> },
    Realloc { ptr: u64, layout: Option<(u64, u64)>, new_size: u64 },
}

/// The four kinds of call, as the report lists them.
#[derive(Clone, Copy, Debug)]
enum CallKind {
    Alloc = 0,
    AllocZeroed = 1,
    Dealloc = 2,
    Realloc = 3,
}

const CALL_KINDS: [(CallKind, &str); 4] = [
    (CallKind::Alloc, "alloc"),
    (CallKind::AllocZeroed, "alloc zeroed"),
    (CallKind::Dealloc, "dealloc"),
    (CallKind::Realloc, "realloc"),
];

impl Call {
    fn kind(&self) -> CallKind {
        match self {
            Call::Alloc { zeroed: false, .. } => CallKind::Alloc,
            Call::Alloc { zeroed: true, .. } => CallKind::AllocZeroed,
            Call::Free { .. } => CallKind::Dealloc,
            Call::Realloc { .. } => CallKind::Realloc,
        }
    }
}

/// Steps and cost of the program at some point, or spent between two points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Measure {
    steps: u64,
    cost: u64,
}

/// What the guest spends in one kind of call.
#[derive(Clone, Copy, Debug, Default)]
struct KindStats {
    calls: u64,
    steps: u64,
    cost: u64,
}

/// A symbol of the ELF that is an allocation entry point.
#[derive(Debug)]
struct Entry {
    pc: u64,
    name: String,
    kind: AllocEntryKind,
    calls: u64,
}

/// A call that has started and has not ended yet. `ra` and `sp` identify the return of a call
/// captured by its symbol.
#[derive(Debug)]
struct Pending {
    entry: usize,
    ra: u64,
    sp: u64,
    call: Call,
    start: Measure,
}

/// A block the guest has allocated and not released yet.
#[derive(Clone, Copy, Debug)]
struct Live {
    size: u64,
    align: u64,
    /// Times the block has been reallocated.
    reallocs: u32,
}

#[derive(Default, Debug, Clone, Copy)]
struct Bucket {
    calls: u64,
    bytes: u64,
}

impl Bucket {
    fn add(&mut self, bytes: u64) {
        self.calls += 1;
        self.bytes += bytes;
    }
}

/// Reallocations that change the size of the block in one direction.
#[derive(Default, Debug, Clone, Copy)]
struct ReallocClass {
    calls: u64,
    /// The allocator returned a different address, so it copied the block.
    moved: u64,
    /// Bytes of the blocks before and after the call.
    old_bytes: u64,
    new_bytes: u64,
    /// Bytes the allocator copied: the smaller of both sizes, for the calls that moved.
    copied_bytes: u64,
}

#[derive(Default, Debug)]
struct ReallocStats {
    calls: u64,
    grow: ReallocClass,
    shrink: ReallocClass,
    same: ReallocClass,
    /// Calls that change the size but not the number of 8-byte words of the block.
    same_words: u64,
    /// Longest chain of reallocations of one block.
    max_chain: u32,
    /// Blocks released (or still live at the end) by the number of reallocations they went
    /// through, in power-of-two buckets indexed by the bit length of that number.
    by_chain: [u64; 32],
}

#[derive(Debug)]
pub struct AllocStats {
    /// Entry points sorted by pc, to find them by binary search. Empty once marks are seen.
    entries: Vec<Entry>,
    min_pc: u64,
    max_pc: u64,
    pending: Option<Pending>,
    /// Allocation marks seen.
    marks: u64,
    /// Calls captured from the symbols before the first mark (a program mixing both sources).
    calls_before_marks: u64,
    /// Layout stated by a `LAYOUT` mark for the call that follows.
    mark_layout: Option<(u64, u64)>,
    /// Call opened by a mark and how many calls were opened inside it.
    mark_open: Option<(Call, Measure)>,
    mark_depth: u32,
    bad_marks: u64,

    kinds: [KindStats; 4],
    live: HashMap<u64, Live>,
    live_bytes: u64,
    peak_live_bytes: u64,
    peak_live_blocks: usize,
    /// Lowest and highest address handed out.
    min_addr: u64,
    high_water: u64,
    allocated_bytes: u64,
    zeroed: Bucket,
    /// Zeroed allocations placed above every address handed out before.
    zeroed_fresh: Bucket,
    frees: u64,
    freed_bytes: u64,
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
    /// Releases and reallocations stating a layout that is not the one of the allocation.
    layout_mismatches: u64,
    /// Blocks asked for, by size in power-of-two buckets indexed by the bit length of the size.
    by_size: [Bucket; 65],
    /// Blocks asked for, by alignment.
    by_align: BTreeMap<u64, Bucket>,
    /// Blocks with an alignment bigger than 8, by size (same buckets as `by_size`).
    over_aligned_by_size: [Bucket; 65],

    trace: Option<BufWriter<File>>,
    trace_error: Option<String>,
}

impl AllocStats {
    /// `symbols` are the functions of the ELF as `(address, demangled name)`.
    pub fn new<'a>(symbols: impl Iterator<Item = (u64, &'a str)>) -> Self {
        let mut entries: Vec<Entry> = symbols
            .filter_map(|(pc, name)| {
                AllocEntryKind::from_symbol(name).map(|kind| Entry {
                    pc,
                    name: name.to_string(),
                    kind,
                    calls: 0,
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
        Self {
            min_pc: entries.first().map_or(u64::MAX, |e| e.pc),
            max_pc: entries.last().map_or(0, |e| e.pc),
            entries,
            pending: None,
            marks: 0,
            calls_before_marks: 0,
            mark_layout: None,
            mark_open: None,
            mark_depth: 0,
            bad_marks: 0,
            kinds: [KindStats::default(); 4],
            live: HashMap::new(),
            live_bytes: 0,
            peak_live_bytes: 0,
            peak_live_blocks: 0,
            min_addr: u64::MAX,
            high_water: 0,
            allocated_bytes: 0,
            zeroed: Bucket::default(),
            zeroed_fresh: Bucket::default(),
            frees: 0,
            freed_bytes: 0,
            reallocs: ReallocStats::default(),
            nested: 0,
            lost: 0,
            null_frees: 0,
            unknown_frees: 0,
            failed_allocs: 0,
            reused: 0,
            layout_mismatches: 0,
            by_size: [Bucket::default(); 65],
            by_align: BTreeMap::new(),
            over_aligned_by_size: [Bucket::default(); 65],
            trace: None,
            trace_error: None,
        }
    }

    /// Writes every call to `path` (`--alloc-trace`), one line each; see [`TRACE_HEADER`].
    pub fn set_trace(&mut self, path: &str) -> std::io::Result<()> {
        let mut trace = BufWriter::new(File::create(path)?);
        trace.write_all(TRACE_HEADER.as_bytes())?;
        self.trace = Some(trace);
        Ok(())
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
        let now = Measure { steps, cost };
        let sp = regs[REG_SP];
        if let Some(pending) = &self.pending {
            if pending.ra == pc && pending.sp == sp {
                let pending = self.pending.take().unwrap();
                self.entries[pending.entry].calls += 1;
                let spent = Measure {
                    steps: now.steps - pending.start.steps,
                    cost: now.cost - pending.start.cost,
                };
                self.on_call(pending.call, regs[REG_A0], spent);
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
        let call = self.entries[entry].kind.decode(a);
        self.pending = Some(Pending { entry, ra, sp, call, start: now });
    }

    /// Called for an allocation mark of kind `kind` with operands `a` and `b`. `steps` and `cost`
    /// are the ones of the program with the mark itself already counted.
    pub fn on_mark(&mut self, kind: u64, a: u64, b: u64, steps: u64, cost: u64) {
        if self.marks == 0 {
            // The program marks its allocator: from here on the symbols are not needed, and
            // watching them would count every call twice.
            self.calls_before_marks = self.kinds.iter().map(|k| k.calls).sum();
            self.pending = None;
            self.min_pc = u64::MAX;
            self.max_pc = 0;
        }
        self.marks += 1;
        let call = match u8::try_from(kind).unwrap_or(0) {
            ALLOC_MARK_ALLOC_ID => Call::Alloc { size: a, align: b, zeroed: false },
            ALLOC_MARK_ALLOC_ZEROED_ID => Call::Alloc { size: a, align: b, zeroed: true },
            ALLOC_MARK_LAYOUT_ID => {
                if self.mark_open.is_none() {
                    self.mark_layout = Some((a, b));
                }
                return;
            }
            ALLOC_MARK_DEALLOC_ID => Call::Free { ptr: a, layout: self.mark_layout.take() },
            ALLOC_MARK_REALLOC_ID => {
                Call::Realloc { ptr: a, layout: self.mark_layout.take(), new_size: b }
            }
            ALLOC_MARK_END_ID => {
                if self.mark_depth > 0 {
                    self.mark_depth -= 1;
                } else if let Some((call, start)) = self.mark_open.take() {
                    // The mark that closes the call is not part of it.
                    let spent = Measure {
                        steps: steps - 1 - start.steps,
                        cost: cost - MAIN_COST - start.cost,
                    };
                    self.on_call(call, a, spent);
                } else {
                    self.bad_marks += 1;
                }
                return;
            }
            _ => {
                self.bad_marks += 1;
                return;
            }
        };
        if self.mark_open.is_some() {
            // An allocator that allocates from itself: the outer call is the one counted.
            self.mark_depth += 1;
            self.nested += 1;
        } else {
            self.mark_open = Some((call, Measure { steps, cost }));
        }
    }

    fn on_call(&mut self, call: Call, result: u64, spent: Measure) {
        let kind = &mut self.kinds[call.kind() as usize];
        kind.calls += 1;
        kind.steps += spent.steps;
        kind.cost += spent.cost;
        self.write_trace(call, result, spent);
        match call {
            Call::Alloc { size, align, zeroed } => self.on_alloc(size, align, zeroed, result),
            Call::Free { ptr, layout } => self.on_free(ptr, layout),
            Call::Realloc { ptr, layout, new_size } => {
                self.on_realloc(ptr, layout, new_size, result)
            }
        }
    }

    fn write_trace(&mut self, call: Call, result: u64, spent: Measure) {
        let Some(trace) = &mut self.trace else {
            return;
        };
        let (steps, cost) = (spent.steps, spent.cost);
        let layout = |layout: Option<(u64, u64)>| match layout {
            Some((size, align)) => format!("{size}\t{align}"),
            None => "-\t-".to_string(),
        };
        let written = match call {
            Call::Alloc { size, align, zeroed } => writeln!(
                trace,
                "{}\t{size}\t{align}\t{result:x}\t{steps}\t{cost}",
                if zeroed { 'z' } else { 'a' }
            ),
            Call::Free { ptr, layout: l } => {
                writeln!(trace, "f\t{ptr:x}\t{}\t{steps}\t{cost}", layout(l))
            }
            Call::Realloc { ptr, layout: l, new_size } => writeln!(
                trace,
                "r\t{ptr:x}\t{}\t{new_size}\t{result:x}\t{steps}\t{cost}",
                layout(l)
            ),
        };
        if let Err(error) = written {
            self.trace_error = Some(error.to_string());
            self.trace = None;
        }
    }

    fn on_alloc(&mut self, size: u64, align: u64, zeroed: bool, ptr: u64) {
        if ptr == 0 {
            self.failed_allocs += 1;
            return;
        }
        if zeroed {
            self.zeroed.add(size);
            if ptr >= self.high_water {
                self.zeroed_fresh.add(size);
            }
        }
        self.insert(ptr, Live { size, align, reallocs: 0 });
    }

    /// Records a block the allocator has just handed out.
    fn insert(&mut self, ptr: u64, block: Live) {
        let bits = (64 - block.size.leading_zeros()) as usize;
        self.by_size[bits].add(block.size);
        self.by_align.entry(block.align).or_default().add(block.size);
        if block.align > 8 {
            self.over_aligned_by_size[bits].add(block.size);
        }
        self.allocated_bytes += block.size;
        if let Some(old) = self.live.insert(ptr, block) {
            self.reused += 1;
            self.live_bytes -= old.size;
            self.count_chain(old.reallocs);
        }
        self.live_bytes += block.size;
        self.peak_live_bytes = self.peak_live_bytes.max(self.live_bytes);
        self.peak_live_blocks = self.peak_live_blocks.max(self.live.len());
        self.min_addr = self.min_addr.min(ptr);
        self.high_water = self.high_water.max(ptr.saturating_add(block.size));
    }

    fn count_chain(&mut self, reallocs: u32) {
        self.reallocs.by_chain[chain_bucket(reallocs)] += 1;
    }

    /// Checks the layout a caller states for a block against the one it was allocated with.
    fn check_layout(&mut self, block: Live, layout: Option<(u64, u64)>) {
        if let Some((size, align)) = layout {
            if size != block.size || (block.align != ALIGN_NOT_STATED && align != block.align) {
                self.layout_mismatches += 1;
            }
        }
    }

    fn on_free(&mut self, ptr: u64, layout: Option<(u64, u64)>) {
        if ptr == 0 {
            self.null_frees += 1;
        } else if let Some(block) = self.live.remove(&ptr) {
            self.check_layout(block, layout);
            self.frees += 1;
            self.freed_bytes += block.size;
            self.live_bytes -= block.size;
            self.count_chain(block.reallocs);
        } else {
            self.unknown_frees += 1;
        }
    }

    fn on_realloc(&mut self, ptr: u64, layout: Option<(u64, u64)>, new_size: u64, new_ptr: u64) {
        let Some(&old) = self.live.get(&ptr) else {
            // realloc(NULL, size) is a malloc; an unknown block can only be counted as one
            if ptr != 0 {
                self.unknown_frees += 1;
            }
            let align = layout.map_or(ALIGN_NOT_STATED, |(_, align)| align);
            return self.on_alloc(new_size, align, false, new_ptr);
        };
        if new_ptr == 0 {
            // C: realloc(ptr, 0) releases the block; otherwise it failed and the block stays
            if new_size == 0 {
                self.on_free(ptr, layout);
            } else {
                self.failed_allocs += 1;
            }
            return;
        }
        self.check_layout(old, layout);
        let stats = &mut self.reallocs;
        stats.calls += 1;
        let class = match new_size.cmp(&old.size) {
            std::cmp::Ordering::Greater => &mut stats.grow,
            std::cmp::Ordering::Less => &mut stats.shrink,
            std::cmp::Ordering::Equal => &mut stats.same,
        };
        class.calls += 1;
        class.old_bytes += old.size;
        class.new_bytes += new_size;
        if new_ptr != ptr {
            class.moved += 1;
            class.copied_bytes += old.size.min(new_size);
        }
        if new_size != old.size && new_size.div_ceil(8) == old.size.div_ceil(8) {
            stats.same_words += 1;
        }
        let reallocs = old.reallocs + 1;
        stats.max_chain = stats.max_chain.max(reallocs);
        // The allocator may return the same address, so the old block leaves the map first.
        self.live.remove(&ptr);
        self.live_bytes -= old.size;
        self.insert(new_ptr, Live { size: new_size, align: old.align, reallocs });
    }

    /// `total_cost` is the variable cost of the whole program (the base cost is not included), the
    /// reference of the `% VAR` column.
    pub fn report(&self, report: &mut StatsReport, total_cost: u64) {
        let n = |v: u64| report.format_number(v);
        let mut out = String::new();
        out += "\nALLOCATOR USAGE\n";
        out += "---------------\n";
        if self.marks > 0 {
            out += &format!(
                "source: {} allocation marks (cost {}, included in the totals of the program)\n",
                n(self.marks),
                n(self.marks * MAIN_COST)
            );
            if self.calls_before_marks > 0 {
                out += &format!(
                    "WARNING: {} calls were captured from the ELF symbols before the first mark\n",
                    n(self.calls_before_marks)
                );
            }
        } else if self.entries.is_empty() {
            out += "No allocation mark and no allocation entry point in the ELF symbols.\n";
            report.add(&out);
            return;
        } else {
            out += "source: ELF symbols (the program has no allocation marks)\n";
            for e in &self.entries {
                out += &format!("  {:>12} calls  {}\n", n(e.calls), e.name);
            }
        }
        if let Some(error) = &self.trace_error {
            out += &format!("WARNING: the trace is incomplete, writing it failed: {error}\n");
        }

        // What the guest spends in its allocator
        out += "\nCALLS (steps and cost between the start and the end of each call)\n";
        let header = format!(
            "{:<16} {:>12} {:>14} {:>16} {:>11} {:>11} {:>8}\n",
            "CALL", "CALLS", "STEPS", "COST", "STEPS/CALL", "COST/CALL", "% VAR"
        );
        out += &header;
        let mut total = KindStats::default();
        let row = |name: &str, k: &KindStats| {
            format!(
                "{:<16} {:>12} {:>14} {:>16} {:>11} {:>11} {:>8}\n",
                name,
                n(k.calls),
                n(k.steps),
                n(k.cost),
                per(k.steps, k.calls),
                per(k.cost, k.calls),
                perc(k.cost, total_cost),
            )
        };
        for (kind, name) in CALL_KINDS {
            let k = &self.kinds[kind as usize];
            total.calls += k.calls;
            total.steps += k.steps;
            total.cost += k.cost;
            out += &row(name, k);
        }
        out += &row("TOTAL", &total);

        // Memory
        out += "\nMEMORY\n";
        out += &format!(
            "allocated {} bytes in {} blocks, released {} bytes in {} blocks\n",
            n(self.allocated_bytes),
            n(self.by_align.values().map(|b| b.calls).sum()),
            n(self.freed_bytes),
            n(self.frees)
        );
        out += &format!(
            "peak live: {} bytes in {} blocks   still live at the end: {} bytes in {} blocks\n",
            n(self.peak_live_bytes),
            n(self.peak_live_blocks as u64),
            n(self.live_bytes),
            n(self.live.len() as u64)
        );
        if self.high_water > 0 {
            let span = self.high_water - self.min_addr;
            out += &format!(
                "addresses handed out: 0x{:x} - 0x{:x} ({} bytes, {:.2}x the peak live)\n",
                self.min_addr,
                self.high_water,
                n(span),
                span as f64 / self.peak_live_bytes.max(1) as f64
            );
        }
        if self.zeroed.calls > 0 {
            let reused = Bucket {
                calls: self.zeroed.calls - self.zeroed_fresh.calls,
                bytes: self.zeroed.bytes - self.zeroed_fresh.bytes,
            };
            out += &format!(
                "zeroed allocations: {} ({} bytes); {} of them ({} bytes) below the highest \
                 address handed out before, memory that may have been used\n",
                n(self.zeroed.calls),
                n(self.zeroed.bytes),
                n(reused.calls),
                n(reused.bytes)
            );
        }
        let ignored = [
            ("free(NULL)", self.null_frees),
            ("free of an unknown block", self.unknown_frees),
            ("failed alloc", self.failed_allocs),
            ("address reused without a free", self.reused),
            ("layout that does not match the allocation", self.layout_mismatches),
            ("nested call", self.nested),
            ("call that never returned", self.lost),
            ("unexpected mark", self.bad_marks),
        ];
        let ignored: Vec<String> = ignored
            .iter()
            .filter(|(_, count)| *count > 0)
            .map(|(name, count)| format!("{name} {}", n(*count)))
            .collect();
        if !ignored.is_empty() {
            out += &format!("anomalies: {}\n", ignored.join(", "));
        }

        // realloc
        let r = &self.reallocs;
        if r.calls > 0 {
            out += "\nREALLOC\n";
            out += &format!(
                "{:<12} {:>12} {:>9} {:>12} {:>12} {:>16} {:>16} {:>16}\n",
                "", "CALLS", "%", "MOVED", "IN PLACE", "OLD BYTES", "NEW BYTES", "COPIED BYTES"
            );
            for (name, c) in [("grow", &r.grow), ("shrink", &r.shrink), ("same size", &r.same)] {
                out += &format!(
                    "{:<12} {:>12} {:>9} {:>12} {:>12} {:>16} {:>16} {:>16}\n",
                    name,
                    n(c.calls),
                    perc(c.calls, r.calls),
                    n(c.moved),
                    n(c.calls - c.moved),
                    n(c.old_bytes),
                    n(c.new_bytes),
                    n(c.copied_bytes)
                );
            }
            if r.grow.calls > 0 {
                out += &format!(
                    "a growing call multiplies the size by {:.2} on average\n",
                    r.grow.new_bytes as f64 / r.grow.old_bytes.max(1) as f64
                );
            }
            if r.same_words > 0 {
                out += &format!(
                    "{} calls change the size without changing the number of 8-byte words\n",
                    n(r.same_words)
                );
            }
            out += &format!("reallocations of one block: up to {}\n", r.max_chain);
            out += &format!("{:<28} {:>12}\n", "REALLOCATIONS", "BLOCKS");
            let mut by_chain = r.by_chain;
            for block in self.live.values() {
                by_chain[chain_bucket(block.reallocs)] += 1;
            }
            for (bits, &blocks) in by_chain.iter().enumerate() {
                if blocks > 0 {
                    out += &format!("{:<28} {:>12}\n", range_label(bits), n(blocks));
                }
            }
        }

        // Alignments
        let total = self.by_align.values().fold(Bucket::default(), |mut acc, b| {
            acc.calls += b.calls;
            acc.bytes += b.bytes;
            acc
        });
        let bucket_row = |label: &str, bucket: &Bucket| {
            format!(
                "{:<28} {:>12} {:>9} {:>16} {:>9}\n",
                label,
                n(bucket.calls),
                perc(bucket.calls, total.calls),
                n(bucket.bytes),
                perc(bucket.bytes, total.bytes)
            )
        };
        let bucket_header = |label: &str| {
            format!("{:<28} {:>12} {:>9} {:>16} {:>9}\n", label, "BLOCKS", "%", "BYTES", "%")
        };
        out += "\nALIGNMENT OF THE BLOCKS (allocations and reallocations)\n";
        out += &bucket_header("ALIGN");
        let mut over = Bucket::default();
        for (&align, bucket) in &self.by_align {
            let label = if align == ALIGN_NOT_STATED {
                "not stated (malloc/new)".to_string()
            } else {
                align.to_string()
            };
            out += &bucket_row(&label, bucket);
            if align > 8 {
                over.calls += bucket.calls;
                over.bytes += bucket.bytes;
            }
        }
        out += &bucket_row("bigger than 8", &over);

        // Sizes
        out += "\nSIZE OF THE BLOCKS (allocations and reallocations)\n";
        out += &bucket_header("BYTES");
        for (bits, bucket) in self.by_size.iter().enumerate() {
            if bucket.calls > 0 {
                out += &bucket_row(&range_label(bits), bucket);
            }
        }
        if over.calls > 0 {
            out += "\nSIZE OF THE BLOCKS WITH AN ALIGNMENT BIGGER THAN 8\n";
            out += &bucket_header("BYTES");
            for (bits, bucket) in self.over_aligned_by_size.iter().enumerate() {
                if bucket.calls > 0 {
                    out += &bucket_row(&range_label(bits), bucket);
                }
            }
        }
        report.add(&out);
    }

    /// Flushes the trace file, if there is one.
    pub fn finish(&mut self) {
        if let Some(trace) = &mut self.trace {
            if let Err(error) = trace.flush() {
                self.trace_error = Some(error.to_string());
            }
        }
    }
}

/// Header of the trace file, which documents its format.
pub const TRACE_HEADER: &str = "\
# ziskemu allocation trace, version 1. One call per line, in execution order, tab separated.
# Pointers are hexadecimal, the rest decimal; '-' is a value the caller does not state, and an
# alignment of 0 is not stated either. A pointer of 0 returned by a call means it failed.
# steps and cost are what the program spent inside the call.
#   a <size> <align> <ptr> <steps> <cost>                             alloc
#   z <size> <align> <ptr> <steps> <cost>                             zeroed alloc
#   f <ptr> <size> <align> <steps> <cost>                             free
#   r <ptr> <old_size> <align> <new_size> <new_ptr> <steps> <cost>    realloc
";

/// Bucket of `by_chain` for a block reallocated `reallocs` times.
fn chain_bucket(reallocs: u32) -> usize {
    ((32 - reallocs.leading_zeros()) as usize).min(31)
}

/// Label of the power-of-two bucket of the values with `bits` significant bits.
fn range_label(bits: usize) -> String {
    match bits {
        0 => "0".to_string(),
        1 => "1".to_string(),
        _ => format!("{}-{}", 1u128 << (bits - 1), (1u128 << bits) - 1),
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
        AllocStats::new(symbols.into_iter())
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

    /// Emits a mark; every mark is a step of cost `MAIN_COST`, and so is everything in between.
    fn mark(stats: &mut AllocStats, step: &mut u64, kind: u8, a: u64, b: u64) {
        *step += 1;
        stats.on_mark(kind as u64, a, b, *step, *step * MAIN_COST);
    }

    fn kind(stats: &AllocStats, kind: CallKind) -> (u64, u64, u64) {
        let k = stats.kinds[kind as usize];
        (k.calls, k.steps, k.cost)
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
        assert_eq!(kind(&stats, CallKind::Alloc), (1, 10, 500));
        assert_eq!((stats.live_bytes, stats.live.len()), (24, 1));
        call(&mut stats, FREE, [0xA000, 0, 0, 0], 0);
        assert_eq!(kind(&stats, CallKind::Dealloc), (1, 10, 500));
        assert_eq!((stats.frees, stats.freed_bytes, stats.live.len()), (1, 24, 0));
        assert_eq!((stats.peak_live_bytes, stats.min_addr, stats.high_water), (24, 0xA000, 0xA018));
    }

    #[test]
    fn a_nested_entry_point_is_not_counted_twice() {
        let mut stats = stats();
        // __rust_alloc_zeroed jumps to __rg_alloc_zeroed (same frame), which calls malloc
        stats.on_step(RUST_ALLOC, &regs(CALLER, SP, [64, 8, 0, 0]), 0, 0);
        stats.on_step(RG_ALLOC, &regs(CALLER, SP, [64, 8, 0, 0]), 1, 10);
        stats.on_step(MALLOC, &regs(RG_ALLOC + 8, SP - 16, [64, 0, 0, 0]), 2, 20);
        stats.on_step(CALLER, &regs(CALLER, SP, [0xB000, 0, 0, 0]), 9, 90);
        assert_eq!(kind(&stats, CallKind::AllocZeroed), (1, 9, 90));
        assert_eq!((stats.zeroed.calls, stats.nested), (1, 2));
        assert_eq!(stats.by_align.get(&8).map(|b| b.calls), Some(1));
    }

    #[test]
    fn a_call_that_never_returns_does_not_hide_the_next_ones() {
        let mut stats = stats();
        stats.on_step(MALLOC, &regs(CALLER, SP - 64, [8, 0, 0, 0]), 0, 0);
        // the stack was unwound past that frame and malloc is called again
        call(&mut stats, MALLOC, [8, 0, 0, 0], 0xA000);
        assert_eq!((stats.kinds[0].calls, stats.lost), (1, 1));
    }

    #[test]
    fn reallocs_are_classified_by_direction_and_by_whether_the_block_moved() {
        let mut stats = stats();
        call(&mut stats, MALLOC, [16, 0, 0, 0], 0xA000);
        call(&mut stats, REALLOC, [0xA000, 40, 0, 0], 0xB000);
        let r = &stats.reallocs;
        assert_eq!((r.calls, r.grow.calls, r.grow.moved, r.grow.copied_bytes), (1, 1, 1, 16));
        assert_eq!((r.grow.old_bytes, r.grow.new_bytes), (16, 40));
        assert!(!stats.live.contains_key(&0xA000));
        assert_eq!(stats.live_bytes, 40);
        // in place: nothing is copied
        call(&mut stats, REALLOC, [0xB000, 400, 0, 0], 0xB000);
        call(&mut stats, REALLOC, [0xB000, 100, 0, 0], 0xB000);
        call(&mut stats, REALLOC, [0xB000, 97, 0, 0], 0xC000);
        call(&mut stats, REALLOC, [0xC000, 97, 0, 0], 0xC000);
        let r = &stats.reallocs;
        assert_eq!((r.grow.calls, r.grow.moved, r.grow.copied_bytes), (2, 1, 16));
        assert_eq!((r.shrink.calls, r.shrink.moved, r.shrink.copied_bytes), (2, 1, 97));
        assert_eq!((r.same.calls, r.same.moved), (1, 0));
        // 100 -> 97 bytes are 13 words both
        assert_eq!((r.same_words, r.max_chain), (1, 5));
        assert_eq!((stats.live_bytes, stats.peak_live_bytes), (97, 400));
        // realloc(NULL) is a malloc and realloc(ptr, 0) a free
        call(&mut stats, REALLOC, [0, 32, 0, 0], 0xD000);
        call(&mut stats, REALLOC, [0xD000, 0, 0, 0], 0);
        assert_eq!((stats.reallocs.calls, stats.frees, stats.live.len()), (5, 1, 1));
    }

    #[test]
    fn marks_describe_whole_calls() {
        let mut stats = stats();
        let mut step = 0;
        mark(&mut stats, &mut step, ALLOC_MARK_ALLOC_ZEROED_ID, 100, 64);
        step += 5; // the allocator works
        mark(&mut stats, &mut step, ALLOC_MARK_END_ID, 0xA000, 0);
        // the five steps in between, without the marks
        assert_eq!(kind(&stats, CallKind::AllocZeroed), (1, 5, 5 * MAIN_COST));
        assert_eq!(stats.live.get(&0xA000).map(|b| (b.size, b.align)), Some((100, 64)));
        assert_eq!((stats.zeroed.calls, stats.zeroed_fresh.calls), (1, 1));
        assert_eq!(stats.over_aligned_by_size[7].calls, 1);

        mark(&mut stats, &mut step, ALLOC_MARK_LAYOUT_ID, 100, 64);
        mark(&mut stats, &mut step, ALLOC_MARK_REALLOC_ID, 0xA000, 300);
        step += 9;
        mark(&mut stats, &mut step, ALLOC_MARK_END_ID, 0xB000, 0);
        assert_eq!(kind(&stats, CallKind::Realloc), (1, 9, 9 * MAIN_COST));
        assert_eq!(stats.live.get(&0xB000).map(|b| (b.size, b.align)), Some((300, 64)));

        mark(&mut stats, &mut step, ALLOC_MARK_LAYOUT_ID, 300, 64);
        mark(&mut stats, &mut step, ALLOC_MARK_DEALLOC_ID, 0xB000, 0);
        mark(&mut stats, &mut step, ALLOC_MARK_END_ID, 0, 0);
        assert_eq!(kind(&stats, CallKind::Dealloc), (1, 0, 0));
        assert_eq!((stats.live.len(), stats.layout_mismatches, stats.bad_marks), (0, 0, 0));

        // a zeroed block below the highest address handed out may be reused memory
        mark(&mut stats, &mut step, ALLOC_MARK_ALLOC_ZEROED_ID, 8, 8);
        mark(&mut stats, &mut step, ALLOC_MARK_END_ID, 0xA000, 0);
        assert_eq!((stats.zeroed.calls, stats.zeroed_fresh.calls), (2, 1));
        assert_eq!(stats.marks, 10);
    }

    #[test]
    fn marks_take_over_from_the_symbols() {
        let mut stats = stats();
        call(&mut stats, MALLOC, [8, 0, 0, 0], 0xA000);
        // a marked call that goes through malloc, which is no longer watched
        let mut step = 200;
        mark(&mut stats, &mut step, ALLOC_MARK_ALLOC_ID, 16, 8);
        assert!(!stats.is_watched(MALLOC));
        mark(&mut stats, &mut step, ALLOC_MARK_END_ID, 0xB000, 0);
        assert_eq!((stats.kinds[0].calls, stats.calls_before_marks), (2, 1));
    }

    #[test]
    fn unbalanced_marks_are_counted_and_ignored() {
        let mut stats = stats();
        let mut step = 0;
        mark(&mut stats, &mut step, ALLOC_MARK_END_ID, 0xA000, 0);
        mark(&mut stats, &mut step, 99, 0, 0);
        assert_eq!(stats.bad_marks, 2);
        // an allocation made from inside the allocator belongs to the outer call
        mark(&mut stats, &mut step, ALLOC_MARK_ALLOC_ID, 64, 8);
        mark(&mut stats, &mut step, ALLOC_MARK_ALLOC_ID, 4096, 8);
        mark(&mut stats, &mut step, ALLOC_MARK_END_ID, 0xF000, 0);
        mark(&mut stats, &mut step, ALLOC_MARK_END_ID, 0xA000, 0);
        assert_eq!((stats.kinds[0].calls, stats.nested), (1, 1));
        assert_eq!(stats.live.get(&0xA000).map(|b| b.size), Some(64));
        // a release stating another size than the allocation
        mark(&mut stats, &mut step, ALLOC_MARK_LAYOUT_ID, 32, 8);
        mark(&mut stats, &mut step, ALLOC_MARK_DEALLOC_ID, 0xA000, 0);
        mark(&mut stats, &mut step, ALLOC_MARK_END_ID, 0, 0);
        assert_eq!((stats.frees, stats.layout_mismatches), (1, 1));
    }
}
