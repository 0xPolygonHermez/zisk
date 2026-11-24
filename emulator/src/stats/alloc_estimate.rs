//! Allocator precompile estimation (`--alloc-stats`, PROTOTYPE).
//!
//! Estimates what the allocator precompile (`precompiles/allocator`) would cost for a program that
//! today runs its own allocator inside the guest, without changing the guest. The calls captured
//! by [`crate::AllocStats`] (allocation marks or ELF symbols) are replayed on shadow heaps that
//! follow the rules of the precompile, to count the rows each variant would need:
//!
//! - `Allocator`: best fit and forced coalescing, one row per operation.
//! - `AllocatorFlex`, eager joins: same heap, but every join is a row of its own.
//! - `AllocatorFlex`, lazy joins: free blocks are only joined when no block fits.
//!
//! The shadow heaps have their own addresses: only the sequence of sizes is taken from the guest.
//!
//! What the estimation assumes:
//! - Sizes are rounded up to 8 bytes, a zero size counts as 8.
//! - An alignment bigger than 8 is served by over-allocating `align` bytes (room to align plus a
//!   word that keeps the start of the block, as the ziskos precompile allocator does).
//! - `realloc` is `alloc` + `memcpy` + `dealloc`.
//! - Each precompile operation costs one main step. The steps of the wrapper around it (rounding
//!   the size, the alignment fix-up) are not counted.
//! - A zeroed allocation only pays a `memset` when the block it gets has been used before.
//!
//! [`AllocPrecompileSummary`] reports instead what the precompile did in an execution that uses it
//! (a guest built with the precompile allocator).

use std::collections::HashMap;

use zisk_core::{
    AllocHeap, AllocInstContext, AllocPolicy, ALLOC_FLEX_ROW_COST, ALLOC_ROW_COST,
    DMA_64_ALIGNED_MEMCPY_COST, DMA_64_ALIGNED_MEMCPY_DIVISOR, DMA_64_ALIGNED_MEMSET_COST,
    DMA_64_ALIGNED_MEMSET_DIVISOR, DMA_MEMCPY_COST, DMA_MEMSET_COST, MAIN_COST,
};

use crate::{StatsReport, MEM_READ_COST, MEM_WRITE_COST};

/// Size of the shadow heaps when the ELF does not tell the size of the guest heap.
const DEFAULT_HEAP_SIZE: u64 = 1 << 31;

const HEAPS: usize = AllocPolicy::ALL.len();

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

/// Size the precompile is asked for: a non-zero multiple of 8, with room to align inside it and
/// to keep the start of the block in front of the aligned address.
fn shadow_size(size: u64, align: u64) -> u64 {
    let size = size.max(1).div_ceil(8) * 8;
    if align > 8 {
        size + align
    } else {
        size
    }
}

/// A block the guest has allocated and not released yet.
#[derive(Debug)]
struct Live {
    /// Size asked to the precompile.
    shadow_size: u64,
    /// Alignment of the block, kept for its reallocations.
    align: u64,
    /// Address of the block in each shadow heap, `None` if that heap was exhausted.
    addr: [Option<u64>; HEAPS],
}

#[derive(Default, Debug)]
struct ReallocEstimate {
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

/// Replay of the calls to the allocator on the heaps of the precompile.
#[derive(Debug)]
pub struct AllocEstimate {
    heaps: [ShadowHeap; HEAPS],
    heap_size: u64,
    live: HashMap<u64, Live>,
    reallocs: ReallocEstimate,
    /// Calls that could not be replayed (a release of a block the replay does not know).
    skipped: u64,
}

impl AllocEstimate {
    /// `heap_size` is the size of the guest heap, 0 if unknown.
    pub fn new(heap_size: u64) -> Self {
        let heap_size = if heap_size == 0 { DEFAULT_HEAP_SIZE } else { heap_size & !7 };
        Self {
            heaps: AllocPolicy::ALL.map(|policy| ShadowHeap::new(policy, heap_size)),
            heap_size,
            live: HashMap::new(),
            reallocs: ReallocEstimate::default(),
            skipped: 0,
        }
    }

    /// An allocation that returned `ptr` (not 0).
    pub fn on_alloc(&mut self, ptr: u64, size: u64, align: u64, zeroed: bool) {
        if self.live.contains_key(&ptr) {
            // the guest released it without the replay seeing it
            self.on_free(ptr);
        }
        let shadow_size = shadow_size(size, align);
        let mut addr = [None; HEAPS];
        for (heap, addr) in self.heaps.iter_mut().zip(addr.iter_mut()) {
            *addr = heap.alloc(shadow_size, zeroed);
        }
        self.live.insert(ptr, Live { shadow_size, align, addr });
    }

    /// A release of the block at `ptr` (not 0).
    pub fn on_free(&mut self, ptr: u64) {
        let Some(live) = self.live.remove(&ptr) else {
            self.skipped += 1;
            return;
        };
        for (heap, addr) in self.heaps.iter_mut().zip(live.addr) {
            if let Some(addr) = addr {
                heap.dealloc(addr);
            }
        }
    }

    /// A reallocation of the block at `ptr` to `new_size` bytes that returned `new_ptr` (neither
    /// is 0). Replayed as alloc + memcpy + dealloc.
    pub fn on_realloc(&mut self, ptr: u64, new_size: u64, new_ptr: u64) {
        let Some(old) = self.live.remove(&ptr) else {
            self.skipped += 1;
            return self.on_alloc(new_ptr, new_size, 0, false);
        };
        let new_shadow_size = shadow_size(new_size, old.align);
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
        // The guest may have returned the same address (it grew in place): the old block is
        // already out of the map.
        self.on_alloc(new_ptr, new_size, old.align, false);
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

    /// `total_cost` is the variable cost of the whole program, `guest_cost` and `guest_calls` what
    /// the allocator of the guest spends today and in how many calls.
    pub fn report(
        &self,
        report: &mut StatsReport,
        total_cost: u64,
        guest_cost: u64,
        guest_calls: u64,
    ) {
        let n = |v: u64| report.format_number(v);
        let mut out = String::new();
        out += &format!(
            "\nALLOCATOR PRECOMPILE ESTIMATION (shadow heap of {} bytes; row cost {} / {})\n",
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
            "COST/CALL",
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
             realloc copies ({}). vs GUEST: against the cost of the calls today ({}).\n",
            n(self.reallocs.copy_cost),
            n(guest_cost)
        );
        if self.skipped > 0 {
            out += &format!("not replayed: {} releases of an unknown block\n", n(self.skipped));
        }
        let r = &self.reallocs;
        if r.calls > 0 {
            // In place there is no copy, and no second row: one operation instead of two.
            let in_place = r.same + r.shrink + r.grow_in_place;
            out += &format!(
                "realloc as alloc + memcpy + dealloc: {} calls (grow {}, {} of them could grow in \
                 place; shrink {}; same size {}), copied {} bytes, cost {}\n",
                n(r.calls),
                n(r.grow),
                n(r.grow_in_place),
                n(r.shrink),
                n(r.same),
                n(r.copied_words * 8),
                n(r.copy_cost)
            );
            out += &format!(
                "an in-place realloc would avoid {} copies ({}), {} bytes, cost {} ({})\n",
                n(in_place),
                perc(in_place, r.calls),
                n(r.avoidable_words * 8),
                n(r.avoidable_cost),
                perc(r.avoidable_cost, r.copy_cost)
            );
        }
        report.add(&out);
    }
}

/// What the allocator precompile did in an execution that uses it (a guest built with the
/// precompile allocator), taken from the emulation context when the execution ends. Its cost is
/// already part of the totals, as variable cost of the `copyb` of its operations.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replays_alloc_free_and_realloc_on_every_heap() {
        let mut estimate = AllocEstimate::new(1 << 20);
        estimate.on_alloc(0x1000, 10, 1, false);
        estimate.on_alloc(0x2000, 64, 64, true);
        estimate.on_realloc(0x1000, 100, 0x3000);
        estimate.on_free(0x2000);
        estimate.on_free(0x3000);
        estimate.on_free(0x4000);
        assert!(estimate.live.is_empty());
        assert_eq!(estimate.skipped, 1);
        assert_eq!(estimate.reallocs.calls, 1);
        assert_eq!(estimate.reallocs.grow, 1);
        for heap in &estimate.heaps {
            // create + 3 allocs + 3 deallocs
            assert_eq!(heap.heap.op_rows, 7);
            assert_eq!(heap.failures, 0);
            assert_eq!(heap.memsets, 0, "the zeroed block is fresh");
        }
    }

    #[test]
    fn shadow_size_rounds_and_makes_room_to_align() {
        assert_eq!(shadow_size(0, 1), 8);
        assert_eq!(shadow_size(9, 8), 16);
        assert_eq!(shadow_size(9, 64), 16 + 64);
    }
}
