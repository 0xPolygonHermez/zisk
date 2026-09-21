//! Filling trace rows in parallel while tallying the multiplicities the fill produces.
//!
//! # Why tally at all
//!
//! Every binary air has to hand `std` a multiplicity per value it looked up — a range for the add
//! airs, a table row for the basic and extension ones. The values themselves are of no further use:
//! they are written into the row and counted, and nothing reads them again.
//!
//! Keeping them to count afterwards is what the obvious shape does, and it scales with the *height*
//! of the instance — one entry per lookup, so `2²³ × 4 × 8` bytes on a full `BinaryAddLarge`,
//! hundreds of megabytes of memory allocated, zeroed and touched to hold values read exactly once.
//!
//! Tallying as the rows are filled scales with the *number of tasks* instead: one histogram per
//! task, merged at the end. That is a few megabytes whatever the air's height, and it costs no extra
//! pass over the data.
//!
//! # Why not increment `std` directly
//!
//! `std` holds its multiplicities in one array shared by every thread, so an increment is an atomic
//! read-modify-write on a line every other thread may want too. That is affordable once per row and
//! ruinous once per *lookup*: `BinaryBasic` looks up one table row per byte, so a full `BinaryHuge`
//! is 16.8M operations × 8 bytes = 134M contended atomics, and the values are far from uniform — a
//! zero byte is common enough that a handful of lines carry most of the traffic.
//!
//! The two shapes here both keep the hot loop free of atomics and touch `std` once at the end:
//!
//! * [`fill_and_tally`] — one dense [`RANGE_16_BITS`] histogram per task, for the add airs, whose
//!   lookups are a 16-bit range.
//! * [`SparseTally`] with [`fill_slots_and_tally`] — a histogram split into [`REGION_ROWS`]-row
//!   regions allocated on demand, for the airs whose lookups index a table far larger than the part
//!   of it any one instance touches (8.8M rows for `BinaryBasic`, 2.5M for `BinaryExtension`).
//!
//! [`SkewedTally`] rides along with the second one, in the same [`FillTally`], for a range check
//! whose value is almost always zero: it counts the zeros and remembers only the values that were
//! not, so a whole instance's worth of them reaches `std` as one call. A task that meets more of
//! them than it keeps stops there and hands back the slots it did not reach, which is the only work
//! that goes back to the caller.
//!
//! [`fill_slots_and_tally`] also walks the chunked inputs with a cursor rather than flattening them,
//! so no `Vec<&BinaryInput>` the size of the instance is built to be read once.

use crate::BinaryInput;
use pil2_std_lib::Std;
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use std::ops::Range;

/// Values a 16-bit range check can take, i.e. the width of one histogram.
pub const RANGE_16_BITS: usize = 0xFFFF + 1;

/// Fills `rows` in parallel, giving each row its slice of `inputs`, and returns the multiplicities
/// the fill tallied.
///
/// `fill` receives one row, the `inputs_per_row` inputs that belong to it, and the histogram of the
/// task it is running on — which it increments directly, one per range-checked chunk it produces.
/// The last row may get a shorter slice when `inputs` does not divide evenly.
///
/// The rows are split into no more chunks than there are rayon threads, so the number of histograms
/// is bounded by the thread count rather than by the finer split rayon would choose on its own.
///
/// # Panics
/// Panics if `rows` does not hold exactly one row per `inputs_per_row` inputs. The two sides are
/// zipped, so a mismatch would silently drop rows (leaving the trace underfilled) or inputs (losing
/// operations), neither of which surfaces until the bus fails to balance. The check is a couple of
/// comparisons per call, not per row, so it is worth keeping in release builds too.
pub fn fill_and_tally<R, T, Fill>(
    rows: &mut [R],
    inputs: &[T],
    inputs_per_row: usize,
    fill: Fill,
) -> Vec<u32>
where
    R: Send,
    T: Sync,
    Fill: Fn(&mut R, &[T], &mut [u32]) + Sync + Send,
{
    assert!(inputs_per_row > 0, "a row must take at least one input");
    assert_eq!(
        rows.len(),
        inputs.len().div_ceil(inputs_per_row),
        "the rows must hold exactly the {} inputs, {inputs_per_row} to a row",
        inputs.len(),
    );

    let tasks = rayon::current_num_threads().max(1);
    let rows_per_task = rows.len().div_ceil(tasks).max(1);

    // `ceil(ceil(n / inputs_per_row) / rows_per_task) == ceil(n / (inputs_per_row * rows_per_task))`,
    // so the two sides of the zip split into the same number of chunks and stay aligned.
    rows.par_chunks_mut(rows_per_task)
        .zip(inputs.par_chunks(rows_per_task * inputs_per_row))
        .map(|(row_chunk, input_chunk)| {
            let mut multiplicities = vec![0u32; RANGE_16_BITS];
            for (row, row_inputs) in row_chunk.iter_mut().zip(input_chunk.chunks(inputs_per_row)) {
                fill(row, row_inputs, &mut multiplicities);
            }
            multiplicities
        })
        .reduce_with(|mut acc, task| {
            for (total, count) in acc.iter_mut().zip(&task) {
                *total += count;
            }
            acc
        })
        .unwrap_or_else(|| vec![0u32; RANGE_16_BITS])
}

/// Bits of a table row that address a slot inside one region of a [`SparseTally`].
pub const REGION_BITS: u32 = 16;

/// Rows one region of a [`SparseTally`] holds.
///
/// Sixteen bits is what makes the split free on the tables this serves: `BinaryBasicTableSM`
/// composes its row as `a + b * 2^8` plus contributions that are all multiples of `2^16`, so a
/// region is exactly one `(opcode, pos_ind, cin, result_is_a)` combination and the low bits are the
/// operand byte pair. Nothing depends on that alignment for correctness — a region is only an
/// allocation unit — but it is what keeps the number of live regions to the combinations an
/// instance actually uses.
pub const REGION_ROWS: usize = 1 << REGION_BITS;

/// Share of a task's lookups a region must carry before it earns a histogram of its own: one in
/// [`PROMOTE_SHARE`]. Below that it is cheaper to remember the rows and hand them to `std` one by one
/// than to allocate, merge and flush [`REGION_ROWS`] counters for a handful of lookups.
///
/// A real opcode mix is steeply skewed — a few of `AND`/`OR`/`XOR`/`ADD`/`SUB`/`LTU` carry almost
/// everything, with a long tail of `MIN`/`MAX`/`SH3ADD`/`LT_ABS` variants barely used — and it is
/// that tail, not the hot regions, that decides whether tallying beats the shared atomics at all.
const PROMOTE_SHARE: usize = 1024;

/// Lookups a region must take before it is promoted, however small the task. Below this even a full
/// region's histogram is not worth its allocation.
const PROMOTE_FLOOR: u32 = 64;

/// A histogram over the rows of a virtual table, allocated a region at a time.
///
/// The tables the binary airs look up have millions of rows, but one instance only ever reaches the
/// regions its opcodes use, and of those only a few carry real traffic. Regions are allocated once
/// they prove they are worth it (see [`PROMOTE_SHARE`]); the rest are remembered row by row.
pub struct SparseTally {
    /// One histogram per region, empty until that region is promoted.
    regions: Vec<Vec<u32>>,

    /// Lookups each region has taken while unpromoted. Meaningless once it is.
    hits: Vec<u32>,

    /// Rows looked up in regions that were never promoted, one entry per lookup.
    ///
    /// Bounded by construction: a region spills at most `promote` rows, so this holds at most
    /// `regions.len() * promote` of them — an eighth of the task's lookups on the 134-region basic
    /// table, a twenty-fifth on the 39-region extension one.
    spill: Vec<u64>,

    /// Lookups a region must take to be promoted.
    promote: u32,

    /// Rows the table has. The last region can reach past it, and what lies there is no row of any
    /// table: [`Self::inc`] refuses it and [`Self::flush`] never hands it to `std`.
    table_rows: u64,
}

impl SparseTally {
    /// An empty tally for a table of `table_rows` rows, expecting about `lookups` of them.
    ///
    /// `lookups` only sets the promotion threshold, so a rough figure is enough: too low and the
    /// cold tail buys histograms it does not fill, too high and the hot regions spill row by row.
    pub fn new(table_rows: u64, lookups: usize) -> Self {
        let regions = (table_rows as usize).div_ceil(REGION_ROWS);
        Self {
            regions: vec![Vec::new(); regions],
            hits: vec![0; regions],
            spill: Vec::new(),
            promote: u32::try_from(lookups / PROMOTE_SHARE).unwrap_or(u32::MAX).max(PROMOTE_FLOOR),
            table_rows,
        }
    }

    /// Counts one lookup of `row`.
    ///
    /// # Panics
    /// Panics in debug builds if `row` is past the table this was built for. Release builds do not
    /// check — this runs once per byte of every operation — and such a row would be counted and then
    /// dropped by [`Self::flush`], so the bus would not balance.
    #[inline(always)]
    pub fn inc(&mut self, row: u64) {
        debug_assert!(
            row < self.table_rows,
            "row {row} is past the {} of the table",
            self.table_rows
        );
        let index = (row >> REGION_BITS) as usize;
        let region = &mut self.regions[index];
        if region.is_empty() {
            self.hits[index] += 1;
            if self.hits[index] < self.promote {
                self.spill.push(row);
                return;
            }
            region.resize(REGION_ROWS, 0);
        }
        region[row as usize & (REGION_ROWS - 1)] += 1;
    }

    /// Adds `other` into this tally. Regions only `other` promoted are moved rather than summed.
    ///
    /// `hits` is not merged: it only governs promotion, which is over once the tasks are done.
    ///
    /// # Panics
    /// Panics if the two were built for tables of different sizes, which would misalign the regions.
    fn merge(mut self, other: Self) -> Self {
        assert_eq!(self.table_rows, other.table_rows, "the tallies are of different tables");
        for (into, from) in self.regions.iter_mut().zip(other.regions) {
            if from.is_empty() {
                continue;
            }
            if into.is_empty() {
                *into = from;
                continue;
            }
            for (total, count) in into.iter_mut().zip(&from) {
                *total += count;
            }
        }
        self.spill.extend(other.spill);
        self
    }

    /// Hands the tallied multiplicities to `std`: each promoted region as one range, then the rows
    /// that never earned one.
    ///
    /// This is where the atomics the fill avoided are paid, from a single thread and once per row of
    /// a promoted region instead of once per lookup.
    pub fn flush<F: PrimeField64>(self, std: &Std<F>, table_id: usize) {
        let table_rows = self.table_rows;
        for (region, counts) in self.regions.into_iter().enumerate() {
            if counts.is_empty() {
                continue;
            }
            // A table whose size is not a whole number of regions leaves the last one hanging over
            // the end. Those slots address no row, so the range handed to `std` stops at the table.
            let start = (region * REGION_ROWS) as u64;
            let len = REGION_ROWS.min((table_rows - start) as usize);
            std.inc_virtual_rows_ranged(table_id, Some(start), &counts[..len]);
        }
        if !self.spill.is_empty() {
            std.inc_virtual_row_batch_one(table_id, &self.spill);
        }
    }

    /// Every row counted, promoted and spilled together. Test-only: the fill never reads counts back.
    #[cfg(test)]
    fn counts(&self) -> std::collections::HashMap<u64, u32> {
        let mut counts = std::collections::HashMap::new();
        for (region, histogram) in self.regions.iter().enumerate() {
            let base = (region * REGION_ROWS) as u64;
            for (row, &count) in histogram.iter().enumerate() {
                if count != 0 {
                    *counts.entry(base + row as u64).or_insert(0) += count;
                }
            }
        }
        for &row in &self.spill {
            *counts.entry(row).or_insert(0) += 1;
        }
        counts
    }
}

/// Values other than zero a [`SkewedTally`] remembers, per task, before it stops counting and hands
/// the rest of its slots back to the caller.
///
/// Sized to be free: one cache line's worth of them per task, so a tally that never reaches the
/// bound costs a counter and a `Vec` that stays within its first allocation. The bound is what makes
/// the buffer safe to keep — an adversarial input cannot make the fill hold one entry per operation,
/// it only makes the caller walk the slots past the bound itself.
pub const SKEWED_OTHERS: usize = 64;

/// A tally for a range check whose value is almost always zero.
///
/// The extension air range-checks the high bits of the shift amount, `(b >> 8) & 0xFFFFFF`. A shift
/// amount is taken modulo 64, so anything a compiler emits has those bits at zero; a non-zero one is
/// a dirty operand, legal but rare. Counting one lookup per shift operation into `std` is therefore
/// millions of contended atomic increments to say "zero" a few million times.
///
/// This counts the zeros instead, and remembers the values that were not — up to [`SKEWED_OTHERS`]
/// of them per task, which is all a real workload ever produces. A task that reaches the bound does
/// not throw away what it counted: it stops at the slot it was on and reports that slot, so
/// [`Self::flush`] hands `std` everything counted before it and the caller looks up only the slots
/// from there to the end of that task's run — its own share of the instance, not the whole of it.
///
/// A slot may be range-checked at most once. The resume point is an operation boundary, so the
/// caller redoes every lookup of every operation it covers; a second lookup on the same operation
/// would be counted twice. Debug builds check it.
#[derive(Default)]
pub struct SkewedTally {
    /// Lookups of zero, up to the slot this task stopped at.
    zeros: u64,

    /// The values that were not zero, one entry per lookup, up to the slot this task stopped at.
    others: Vec<u64>,

    /// First slot of the row the fill is on, so a lookup can name the slot it came from.
    row_slot: usize,

    /// The slot this task stopped counting at, once it reached [`SKEWED_OTHERS`]. Taken by
    /// [`Self::finish`], which is what turns it into a range.
    stopped_at: Option<usize>,

    /// Slot ranges nobody counted, one per task that reached the bound.
    ///
    /// Bounded by the thread count, and disjoint: a task only ever names slots inside its own run,
    /// and stops counting at the first one it names.
    unfinished: Vec<Range<usize>>,

    /// The slot of the last lookup counted, so a second lookup on the same slot — which the resume
    /// point cannot express — is caught rather than silently double-counted.
    #[cfg(debug_assertions)]
    last_slot: Option<usize>,
}

impl SkewedTally {
    /// Points the tally at the row starting at global slot `first_slot`.
    ///
    /// Called once per row by the fill, which is the only place that knows where a row sits in the
    /// instance; [`Self::inc`] then names its slot with the lane it was given.
    #[inline(always)]
    pub fn at_row(&mut self, first_slot: usize) {
        self.row_slot = first_slot;
    }

    /// Counts one range check of `value`, looked up by the operation in `lane` of the current row.
    #[inline(always)]
    pub fn inc(&mut self, lane: usize, value: u64) {
        #[cfg(debug_assertions)]
        {
            let slot = self.row_slot + lane;
            debug_assert!(
                self.last_slot != Some(slot),
                "slot {slot} was range checked twice; the resume point cannot express that",
            );
            self.last_slot = Some(slot);
        }
        if self.stopped_at.is_some() {
            return;
        }
        if value == 0 {
            self.zeros += 1;
            return;
        }
        if self.others.len() == SKEWED_OTHERS {
            // This lookup is not counted, so the slot it came from is where the caller resumes.
            self.stopped_at = Some(self.row_slot + lane);
            return;
        }
        self.others.push(value);
    }

    /// Closes this task's run, which ends at global slot `end`.
    ///
    /// A task that stopped counting hands back the slots from where it stopped to `end`; one that
    /// counted its whole run hands back nothing.
    fn finish(&mut self, end: usize) {
        if let Some(start) = self.stopped_at.take() {
            self.unfinished.push(start..end);
        }
    }

    /// Adds `other` into this tally.
    ///
    /// The [`SKEWED_OTHERS`] bound is per task, so merging tasks can carry more than it — at most
    /// one buffer per task, which is the same few kilobytes the tasks already held.
    fn merge(mut self, other: Self) -> Self {
        debug_assert!(
            self.stopped_at.is_none() && other.stopped_at.is_none(),
            "a task was merged before `finish` turned where it stopped into a range",
        );
        self.zeros += other.zeros;
        self.others.extend(other.others);
        self.unfinished.extend(other.unfinished);
        self
    }

    /// The zero count, the values it kept and the slots it left — what [`Self::flush`] would hand to
    /// `std` and give back. Test-only: the fill never reads the counts back.
    #[cfg(test)]
    fn counted(&self) -> (u64, Vec<u64>, Vec<Range<usize>>) {
        (self.zeros, self.others.clone(), self.unfinished.clone())
    }

    /// Hands the counted range checks to `std` and gives back the slots nobody counted.
    ///
    /// The returned ranges are disjoint and hold no counts: the caller has to look up every range
    /// check of every operation they cover, which [`for_each_operation_in`] walks. They are empty on
    /// any workload that stays under [`SKEWED_OTHERS`] dirty values per task.
    #[must_use]
    pub fn flush<F: PrimeField64>(self, std: &Std<F>, range_id: usize) -> Vec<Range<usize>> {
        if self.zeros > 0 {
            std.range_check(range_id, 0u64, self.zeros);
        }
        if !self.others.is_empty() {
            std.range_check_batch_one(range_id, &self.others);
        }
        self.unfinished
    }
}

/// The counters one task of a fill writes into: the table rows it looks up, and the values it range
/// checks.
///
/// Both are per task and merged at the end, which is what keeps the fill free of atomics; see the
/// module documentation. An air that range-checks nothing simply never touches [`Self::range`],
/// which then costs it a `u64` and an unallocated `Vec` per task.
pub struct FillTally {
    /// Multiplicities of the table rows the fill looked up.
    pub table: SparseTally,

    /// The values the fill range-checked, one lookup per operation.
    pub range: SkewedTally,
}

impl FillTally {
    /// An empty tally for a table of `table_rows` rows, expecting about `lookups` of them.
    fn new(table_rows: u64, lookups: usize) -> Self {
        Self { table: SparseTally::new(table_rows, lookups), range: SkewedTally::default() }
    }

    /// Counts one lookup of table row `row`. See [`SparseTally::inc`].
    #[inline(always)]
    pub fn inc(&mut self, row: u64) {
        self.table.inc(row);
    }

    /// Counts one range check of `value`, by the operation in `lane`. See [`SkewedTally::inc`].
    #[inline(always)]
    pub fn inc_range(&mut self, lane: usize, value: u64) {
        self.range.inc(lane, value);
    }

    /// Adds `other` into this tally, counter by counter.
    fn merge(self, other: Self) -> Self {
        Self { table: self.table.merge(other.table), range: self.range.merge(other.range) }
    }
}

/// Walks a chunked input list from an arbitrary global offset, without flattening it.
///
/// The fill is parallel over rows and a row's operations can straddle a chunk boundary, which is
/// what a flattened `Vec<&BinaryInput>` was there to hide. A task takes a contiguous run of rows,
/// hence a contiguous run of operations, so walking forward from where its run starts gives the same
/// sequence with nothing materialized.
struct InputCursor<'a> {
    chunks: &'a [Vec<BinaryInput>],
    chunk: usize,
    offset: usize,
}

impl<'a> InputCursor<'a> {
    /// A cursor positioned at global operation `slot`.
    ///
    /// `starts` is the prefix sum of the chunk lengths, `starts[i]` being the global index of the
    /// first operation of chunk `i` and `starts[chunks.len()]` the total.
    fn new(chunks: &'a [Vec<BinaryInput>], starts: &[usize], slot: usize) -> Self {
        // The chunk `slot` falls in is the last one that starts at or before it. Empty chunks share
        // a start with their successor, and landing on one is harmless: `next` walks past them.
        let chunk = starts.partition_point(|&start| start <= slot) - 1;
        Self { chunks, chunk, offset: slot - starts[chunk] }
    }

    #[inline(always)]
    fn next(&mut self) -> Option<&'a BinaryInput> {
        while self.chunk < self.chunks.len() {
            let chunk = &self.chunks[self.chunk];
            if self.offset < chunk.len() {
                let input = &chunk[self.offset];
                self.offset += 1;
                return Some(input);
            }
            self.chunk += 1;
            self.offset = 0;
        }
        None
    }
}

/// Calls `visit` on every operation the slot ranges in `slots` cover, in order.
///
/// `slots` is what [`SkewedTally::flush`] gave back: the runs of operations no task counted, which
/// the caller has to look up itself. They are walked with the same cursor the fill uses, so this
/// reads the chunks in place and touches nothing outside the ranges.
///
/// # Panics
/// Panics if a range reaches past the operations `inputs` holds.
pub fn for_each_operation_in<Visit>(
    inputs: &[Vec<BinaryInput>],
    slots: &[Range<usize>],
    mut visit: Visit,
) where
    Visit: FnMut(&BinaryInput),
{
    if slots.is_empty() {
        return;
    }
    let starts = chunk_starts(inputs);
    for range in slots {
        let mut cursor = InputCursor::new(inputs, &starts, range.start);
        for slot in range.clone() {
            let input =
                cursor.next().unwrap_or_else(|| panic!("slot {slot} is past the operations"));
            visit(input);
        }
    }
}

/// Prefix sum of the chunk lengths, one entry longer than `chunks`.
fn chunk_starts(chunks: &[Vec<BinaryInput>]) -> Vec<usize> {
    let mut starts = Vec::with_capacity(chunks.len() + 1);
    let mut total = 0;
    starts.push(0);
    for chunk in chunks {
        total += chunk.len();
        starts.push(total);
    }
    starts
}

/// Fills `rows` in parallel from the chunked `inputs`, tallying the table rows the fill looks up and
/// the values it range checks.
///
/// Slots are numbered consecutively across the whole instance, `lanes_x_row` to a row, so row `r`
/// takes operations `r * lanes_x_row ..`. `slot` fills one of them and counts what it looks up;
/// `pad` fills a slot of the last row that has no operation left — only the last row can be short,
/// and the trace buffer comes from a pool and is not zeroed, so those slots cannot be left alone.
///
/// `lookups_x_slot` is how many table rows `slot` counts per operation — eight for the airs that
/// look one up per byte. It only sizes the promotion threshold of each task's [`SparseTally`], so an
/// approximation is fine.
///
/// The rows are split into no more chunks than there are rayon threads, so the number of live
/// [`FillTally`] histograms is bounded by the thread count.
///
/// # Panics
/// Panics if `rows` does not hold exactly the `total_inputs` operations at `lanes_x_row` to a row.
#[allow(clippy::too_many_arguments)]
pub fn fill_slots_and_tally<R, Slot, Pad>(
    rows: &mut [R],
    inputs: &[Vec<BinaryInput>],
    total_inputs: usize,
    lanes_x_row: usize,
    table_rows: u64,
    lookups_x_slot: usize,
    slot: Slot,
    pad: Pad,
) -> FillTally
where
    R: Send,
    Slot: Fn(&mut R, usize, &BinaryInput, &mut FillTally) + Sync + Send,
    Pad: Fn(&mut R, usize) + Sync + Send,
{
    assert!(lanes_x_row > 0, "a row must take at least one operation");
    assert_eq!(
        rows.len(),
        total_inputs.div_ceil(lanes_x_row),
        "the rows must hold exactly the {total_inputs} operations, {lanes_x_row} to a row",
    );

    let starts = chunk_starts(inputs);
    assert_eq!(
        starts[inputs.len()],
        total_inputs,
        "the chunks hold {} operations, not the {total_inputs} announced",
        starts[inputs.len()],
    );

    let tasks = rayon::current_num_threads().max(1);
    let rows_per_task = rows.len().div_ceil(tasks).max(1);
    let lookups_x_task = rows_per_task * lanes_x_row * lookups_x_slot;

    rows.par_chunks_mut(rows_per_task)
        .enumerate()
        .map(|(task, row_chunk)| {
            let mut tally = FillTally::new(table_rows, lookups_x_task);
            let mut done = task * rows_per_task * lanes_x_row;
            let mut cursor = InputCursor::new(inputs, &starts, done);

            for row in row_chunk.iter_mut() {
                let filled = lanes_x_row.min(total_inputs - done);
                tally.range.at_row(done);
                for lane in 0..filled {
                    let input = cursor.next().expect("the cursor holds one input per filled slot");
                    slot(row, lane, input, &mut tally);
                }
                for lane in filled..lanes_x_row {
                    pad(row, lane);
                }
                done += filled;
            }
            // `done` is now the end of this task's run, which is what closes the slots it left.
            tally.range.finish(done);
            tally
        })
        .reduce_with(FillTally::merge)
        .unwrap_or_else(|| FillTally::new(table_rows, lookups_x_task))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chunk lengths that exercise the cursor: empty chunks at both ends and in the middle, chunks
    /// shorter and longer than a row, and a total that does not divide by the packing.
    const CHUNKINGS: &[&[usize]] = &[
        &[],
        &[0],
        &[1],
        &[0, 0, 5, 0, 0],
        &[3, 1, 4, 1, 5, 9, 2, 6],
        &[1, 1, 1, 1, 1, 1, 1],
        &[100, 0, 1, 37],
        &[1000, 1, 1000],
    ];

    /// The chunkings above stay well under one region, so a fill can count operation `i` into row
    /// `i` and read the count back without folding.
    const _: () = assert!(2001 < REGION_ROWS);

    fn chunked(lengths: &[usize]) -> Vec<Vec<BinaryInput>> {
        let mut next = 0u64;
        lengths
            .iter()
            .map(|&len| {
                (0..len)
                    .map(|_| {
                        next += 1;
                        // `a` carries the operation's global index, so the fill can check the order.
                        BinaryInput::new(0, next - 1, 0)
                    })
                    .collect()
            })
            .collect()
    }

    /// The tally must match a plain serial histogram, whatever the split and whatever regions the
    /// rows fall in — including rows the fill never touches, and the very last row of the table.
    ///
    /// Both promotion regimes are covered: `lookups` far above the row count promotes nothing (every
    /// row spills), far below it promotes on first touch, and the middle mixes the two.
    #[test]
    fn the_sparse_tally_matches_a_serial_count() {
        const TABLE_ROWS: u64 = 8_781_824;
        // A skewed mix: one region takes most of the rows, the rest are scattered over the table.
        let rows: Vec<u64> = (0..40_000u64)
            .map(|i| if i % 3 == 0 { i % 5_000 } else { (i * 2_654_435_761) % TABLE_ROWS })
            .chain([0, TABLE_ROWS - 1, REGION_ROWS as u64, REGION_ROWS as u64 - 1])
            .collect();

        let mut expected = std::collections::HashMap::new();
        for &row in &rows {
            *expected.entry(row).or_insert(0u32) += 1;
        }

        for lookups in [0, 40_000, 400_000, 40_000_000] {
            // Split the same rows over several tallies and merge them, which is what the fill does.
            let mut merged: Option<SparseTally> = None;
            for part in rows.chunks(rows.len().div_ceil(7)) {
                let mut tally = SparseTally::new(TABLE_ROWS, lookups);
                for &row in part {
                    tally.inc(row);
                }
                merged = Some(match merged {
                    None => tally,
                    Some(acc) => acc.merge(tally),
                });
            }
            let merged = merged.expect("the rows are not empty");

            // Equal as maps: every row handed in is counted right, and no row that was not
            // handed in is counted at all.
            assert_eq!(merged.counts(), expected, "lookups {lookups}");
        }
    }

    /// The spill is what keeps a cold region from buying a histogram, so it must stay small: a region
    /// spills at most `promote` rows, and there are only so many regions.
    #[test]
    fn the_spill_is_bounded_by_the_promotion_threshold() {
        const TABLE_ROWS: u64 = 8_781_824;
        let lookups = 1_000_000;
        let mut tally = SparseTally::new(TABLE_ROWS, lookups);
        let promote = tally.promote as usize;
        let regions = tally.regions.len();

        // Spread the lookups over every region, which is the worst case for the spill.
        for i in 0..lookups as u64 {
            tally.inc((i % regions as u64) * REGION_ROWS as u64 + i % 251);
        }
        assert!(
            tally.spill.len() <= regions * promote,
            "spilled {} rows, more than the {regions} regions x {promote} bound",
            tally.spill.len(),
        );
        // Uniform traffic over every region promotes them all, so nothing is left spilling.
        assert!(tally.regions.iter().all(|region| !region.is_empty()), "a region stayed cold");
    }

    /// A row past the table would be dropped by the flush, so the fill must not produce one. The
    /// interesting case is a table that does not end on a region boundary — `BinaryExtension`'s does
    /// not — where the row still lands inside an allocated region and nothing else would catch it.
    #[test]
    #[should_panic(expected = "is past the")]
    fn a_row_past_a_ragged_table_is_rejected() {
        let ragged = REGION_ROWS as u64 + 7;
        assert_ne!(ragged % REGION_ROWS as u64, 0, "the table must not end on a region boundary");
        let mut tally = SparseTally::new(ragged, 0);
        tally.inc(ragged);
    }

    /// The tables this serves: the basic one tiles regions exactly, the extension one does not, and
    /// the flush has to stop at the table either way.
    #[test]
    fn the_flush_range_stops_at_the_table() {
        for table_rows in [
            crate::BinaryBasicTableSM::TABLE_ROWS,
            crate::BinaryExtensionTableSM::TABLE_ROWS,
            REGION_ROWS as u64,
            REGION_ROWS as u64 + 1,
            1,
        ] {
            let tally = SparseTally::new(table_rows, 0);
            let regions = tally.regions.len() as u64;
            assert!(
                (regions - 1) * (REGION_ROWS as u64) < table_rows,
                "{table_rows} rows got a region that holds nothing"
            );

            // What `flush` would hand to `std`, region by region, must cover every row exactly once
            // and stop at the last one.
            let mut covered = 0u64;
            for region in 0..regions {
                let start = region * REGION_ROWS as u64;
                covered += (REGION_ROWS as u64).min(table_rows - start);
            }
            assert_eq!(covered, table_rows, "{table_rows} rows");
        }
    }

    /// The whole point of dropping the flatten: every operation must still reach exactly one slot,
    /// in order, whatever the chunking and however rayon splits the rows.
    #[test]
    fn every_operation_reaches_its_slot_in_order() {
        for lengths in CHUNKINGS {
            for lanes_x_row in [1usize, 2, 3, 4, 8] {
                let inputs = chunked(lengths);
                let total: usize = lengths.iter().sum();
                let rows_used = total.div_ceil(lanes_x_row);

                // Each row records the global index of every operation it was given, and `u64::MAX`
                // for the slots the padding closure filled.
                let mut rows = vec![vec![u64::MAX; lanes_x_row]; rows_used];

                let tally = fill_slots_and_tally(
                    &mut rows,
                    &inputs,
                    total,
                    lanes_x_row,
                    REGION_ROWS as u64,
                    1,
                    |row, lane, input, tally| {
                        row[lane] = input.a;
                        tally.inc(input.a % REGION_ROWS as u64);
                    },
                    |row, lane| row[lane] = u64::MAX,
                );

                let seen: Vec<u64> =
                    rows.iter().flatten().copied().filter(|&v| v != u64::MAX).collect();
                assert_eq!(
                    seen,
                    (0..total as u64).collect::<Vec<_>>(),
                    "{lengths:?} at {lanes_x_row} lanes"
                );

                // Only the last row may be padded, and only past the operations it holds.
                let padded: usize = rows.iter().flatten().filter(|&&v| v == u64::MAX).count();
                assert_eq!(
                    padded,
                    rows_used * lanes_x_row - total,
                    "{lengths:?} at {lanes_x_row} lanes"
                );

                // The tally survived the merge across tasks: one count per operation, and the
                // fill counted operation `i` into row `i` (the chunkings stay under one region).
                assert_eq!(
                    tally.table.counts(),
                    (0..total as u64).map(|index| (index, 1)).collect(),
                    "{lengths:?} at {lanes_x_row} lanes"
                );
            }
        }
    }

    /// The cursor must land on the right operation for any starting slot, which is what lets a task
    /// begin in the middle of a chunk.
    #[test]
    fn the_cursor_starts_at_any_slot() {
        for lengths in CHUNKINGS {
            let inputs = chunked(lengths);
            let starts = chunk_starts(&inputs);
            let total: usize = lengths.iter().sum();
            assert_eq!(starts[inputs.len()], total, "{lengths:?}");

            for slot in 0..=total {
                let mut cursor = InputCursor::new(&inputs, &starts, slot);
                for expected in slot..total {
                    let input = cursor.next().expect("an operation is left");
                    assert_eq!(input.a, expected as u64, "{lengths:?} from slot {slot}");
                }
                assert!(cursor.next().is_none(), "{lengths:?} from slot {slot}: past the end");
            }
        }
    }

    /// The tally must match a plain serial histogram of the same chunks, whatever the split.
    #[test]
    fn the_tally_matches_a_serial_count() {
        for inputs_per_row in [1usize, 3, 5] {
            for count in [0usize, 1, 7, 1000] {
                let inputs: Vec<u64> = (0..count as u64).map(|i| (i * 7) % 300).collect();
                let rows = count.div_ceil(inputs_per_row);
                let mut filled = vec![0u64; rows];

                let multiplicities =
                    fill_and_tally(&mut filled, &inputs, inputs_per_row, |row, row_inputs, m| {
                        *row = row_inputs.len() as u64;
                        for &input in row_inputs {
                            m[input as usize] += 1;
                        }
                    });

                let mut expected = vec![0u32; RANGE_16_BITS];
                for &input in &inputs {
                    expected[input as usize] += 1;
                }
                assert_eq!(multiplicities, expected, "{count} inputs, {inputs_per_row} per row");

                // And every row was visited, with the inputs that belong to it.
                assert_eq!(filled.iter().sum::<u64>(), count as u64);
            }
        }
    }

    /// The split never asks for more chunks than there are threads, which is what bounds how many
    /// histograms are alive at once. It can be fewer — there is no work to give every thread when
    /// the rows are few.
    #[test]
    fn the_split_never_exceeds_the_thread_count() {
        let threads = rayon::current_num_threads().max(1);
        for rows in [1usize, 2, threads - 1, threads, threads + 1, 7 * threads + 3, 100_000] {
            let rows_per_task = rows.div_ceil(threads).max(1);
            assert!(
                rows.div_ceil(rows_per_task) <= threads,
                "{rows} rows split into {} chunks, more than the {threads} threads",
                rows.div_ceil(rows_per_task),
            );
        }
    }

    /// Rows and inputs that do not line up would silently drop one or the other, so the contract is
    /// checked rather than trusted.
    #[test]
    #[should_panic(expected = "the rows must hold exactly")]
    fn mismatched_rows_and_inputs_are_an_error() {
        // Ten inputs three to a row need four rows, not three.
        fill_and_tally(&mut [0u64; 3], &[0u64; 10], 3, |_, _, _| {});
    }

    /// No work means an all-zero tally rather than a panic on the empty reduction.
    #[test]
    fn nothing_to_fill_tallies_nothing() {
        let multiplicities = fill_and_tally(&mut [0u64; 0], &[0u64; 0], 4, |_, _, m| m[1] += 1);
        assert_eq!(multiplicities.len(), RANGE_16_BITS);
        assert!(multiplicities.iter().all(|&m| m == 0));
    }

    /// Counts `values` into a tally as one task's run of one-lane rows, ending at slot `end`.
    fn skewed(values: &[u64], end: usize) -> SkewedTally {
        let mut tally = SkewedTally::default();
        for (slot, &value) in values.iter().enumerate() {
            tally.at_row(slot);
            tally.inc(0, value);
        }
        tally.finish(end);
        tally
    }

    /// The case the skewed tally exists for: a run of lookups that are all zero costs `std` one
    /// call, the few that are not are kept as they came, and nothing is left for the caller.
    #[test]
    fn the_skewed_tally_separates_the_zeros_from_the_rest() {
        assert_eq!(skewed(&[], 0).counted(), (0, vec![], vec![]), "an untouched tally is empty");

        let tally = skewed(&[0, 0, 7, 0, 0x123456, 0, 0], 7);
        assert_eq!(tally.counted(), (5, vec![7, 0x123456], vec![]));
    }

    /// Tasks tally independently and are merged, so what the merge produces has to be what one task
    /// would have counted on its own.
    #[test]
    fn merging_skewed_tallies_adds_both_sides() {
        let merged = skewed(&[0, 0, 9], 3).merge(skewed(&[0, 4, 0, 0], 4));
        assert_eq!(merged.counted(), (5, vec![9, 4], vec![]));

        // The per-task bound is not a bound on the merge: two full tasks merge into one tally that
        // holds both their buffers, which is still only a buffer per task.
        let full = vec![1u64; SKEWED_OTHERS];
        let merged = skewed(&full, full.len()).merge(skewed(&full, full.len()));
        assert_eq!(merged.counted(), (0, vec![1; 2 * SKEWED_OTHERS], vec![]));
    }

    /// Past the bound the tally stops counting rather than growing with the instance, but it keeps
    /// what it counted: only the slots from where it stopped to the end of its run go back to the
    /// caller, and they survive a merge with a task that counted its whole run.
    // A one-element `Vec`/array of `Range` is what a single task that stopped produces; the
    // lint's suggestion to `collect` the range is the opposite of what these assert.
    #[allow(clippy::single_range_in_vec_init)]
    #[test]
    fn a_skewed_tally_that_stops_hands_back_only_what_is_left() {
        // The bound is reached on the slot after the last value it keeps, and the run is longer.
        let mut values = vec![3u64; SKEWED_OTHERS + 1];
        values.extend([0, 0, 5]);
        let tally = skewed(&values, 1000);

        assert_eq!(
            tally.counted(),
            (0, vec![3; SKEWED_OTHERS], vec![SKEWED_OTHERS..1000]),
            "the kept values stand and the rest of the run goes back",
        );

        // A task that stopped and one that did not merge into the counts of both plus the one range.
        let merged = skewed(&[0, 0, 8], 3).merge(tally);
        let kept = [vec![8u64], vec![3; SKEWED_OTHERS]].concat();
        assert_eq!(merged.counted(), (2, kept, vec![SKEWED_OTHERS..1000]));
    }

    /// A slot the tally counted must not also come back in a range, or its lookup is counted twice.
    #[test]
    fn the_slots_handed_back_start_where_the_counting_stopped() {
        let values = vec![7u64; SKEWED_OTHERS + 20];
        let tally = skewed(&values, values.len());
        let (_, kept, unfinished) = tally.counted();

        assert_eq!(unfinished, vec![SKEWED_OTHERS..values.len()]);
        assert_eq!(
            kept.len() + unfinished.iter().map(|range| range.len()).sum::<usize>(),
            values.len(),
            "every slot is either counted or handed back, and none is both",
        );
    }

    /// The fill hands the closure one tally holding both counters, and both have to come back
    /// merged across however many tasks rayon used.
    #[test]
    fn the_fill_tallies_the_range_checks_too() {
        for lengths in CHUNKINGS {
            let inputs = chunked(lengths);
            let total: usize = lengths.iter().sum();
            let mut rows = vec![0u64; total];

            // One operation in 101 range checks its own index and the rest check zero, sparse enough
            // that the largest chunking stays under the per-task bound however few threads run it.
            let tally = fill_slots_and_tally(
                &mut rows,
                &inputs,
                total,
                1,
                REGION_ROWS as u64,
                1,
                |row, lane, input, tally| {
                    *row = input.a;
                    tally.inc(input.a % REGION_ROWS as u64);
                    tally.inc_range(lane, if input.a % 101 == 100 { input.a } else { 0 });
                },
                |_, _| unreachable!("one operation to a row leaves nothing to pad"),
            );

            let expected: Vec<u64> = (0..total as u64).filter(|index| index % 101 == 100).collect();
            let (zeros, mut kept, unfinished) = tally.range.counted();
            kept.sort_unstable();
            assert_eq!((zeros, kept), ((total - expected.len()) as u64, expected), "{lengths:?}");
            assert!(unfinished.is_empty(), "{lengths:?}: far too few to reach the bound");
        }
    }

    /// The slots a task hands back have to name the operations it did not count, so walking them
    /// with [`for_each_operation_in`] finds exactly those — whatever the chunking and the split.
    #[test]
    fn the_fill_hands_back_the_slots_it_did_not_count() {
        for lengths in CHUNKINGS {
            let inputs = chunked(lengths);
            let total: usize = lengths.iter().sum();
            let mut rows = vec![0u64; total];

            // Every value is non-zero, so every task stops after the first SKEWED_OTHERS of its run.
            let tally = fill_slots_and_tally(
                &mut rows,
                &inputs,
                total,
                1,
                REGION_ROWS as u64,
                1,
                |row, lane, input, tally| {
                    *row = input.a;
                    tally.inc(input.a % REGION_ROWS as u64);
                    tally.inc_range(lane, input.a + 1);
                },
                |_, _| unreachable!("one operation to a row leaves nothing to pad"),
            );

            let (zeros, kept, unfinished) = tally.range.counted();
            assert_eq!(zeros, 0, "{lengths:?}");

            // Whatever the split, each operation was either kept or handed back, never both.
            let mut walked = Vec::new();
            for_each_operation_in(&inputs, &unfinished, |input| walked.push(input.a + 1));
            let mut seen = [kept, walked].concat();
            seen.sort_unstable();
            assert_eq!(seen, (1..=total as u64).collect::<Vec<_>>(), "{lengths:?}");
        }
    }

    /// The ranges are walked in place from an arbitrary slot, which is the whole point of reusing
    /// the cursor: an empty list walks nothing, and disjoint ranges walk their own operations.
    // A one-element `Vec`/array of `Range` is what a single task that stopped produces; the
    // lint's suggestion to `collect` the range is the opposite of what these assert.
    #[allow(clippy::single_range_in_vec_init)]
    #[test]
    fn walking_slot_ranges_visits_exactly_them() {
        for lengths in CHUNKINGS {
            let inputs = chunked(lengths);
            let total: usize = lengths.iter().sum();

            let mut walked = Vec::new();
            for_each_operation_in(&inputs, &[], |input| walked.push(input.a));
            assert!(walked.is_empty(), "{lengths:?}: no ranges, no operations");

            for_each_operation_in(&inputs, &[0..total], |input| walked.push(input.a));
            assert_eq!(walked, (0..total as u64).collect::<Vec<_>>(), "{lengths:?}: the whole run");

            // Two disjoint ranges, the second starting past the first, as a merge produces them.
            if total >= 4 {
                let (first, second) = (1..total / 2, total / 2 + 1..total);
                let mut walked = Vec::new();
                for_each_operation_in(&inputs, &[first.clone(), second.clone()], |input| {
                    walked.push(input.a as usize)
                });
                assert_eq!(walked, first.chain(second).collect::<Vec<_>>(), "{lengths:?}");
            }
        }
    }
}
