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
//! * [`fill_and_tally_chunked`] — one dense [`RANGE_16_BITS`] histogram per task, for the add airs,
//!   whose lookups are a 16-bit range.
//! * [`SparseTally`] with [`fill_slots_and_tally`] — a histogram split into [`REGION_ROWS`]-row
//!   regions allocated on demand, for the airs whose lookups index a table far larger than the part
//!   of it any one instance touches (8.8M rows for `BinaryBasic`, 2.5M for `BinaryExtension`).
//!
//! Both walk the chunked inputs with a cursor rather than flattening them, so no
//! `Vec<&BinaryInput>` the size of the instance is built to be read once.

use crate::BinaryInput;
use pil2_std_lib::Std;
use proofman_fields::PrimeField64;
use rayon::prelude::*;

/// Values a 16-bit range check can take, i.e. the width of one histogram.
pub const RANGE_16_BITS: usize = 0xFFFF + 1;

/// Fills `rows` in parallel from the chunked `inputs`, giving each row the operations that belong
/// to it, and returns the multiplicities the fill tallied.
///
/// `fill` receives one row, its `inputs_per_row` operations — fewer on the last row, when
/// `total_inputs` does not divide evenly — and the histogram of the task it is running on, which it
/// increments directly, one per range-checked chunk it produces.
///
/// The rows are split into no more chunks than there are rayon threads, so the number of histograms
/// is bounded by the thread count rather than by the finer split rayon would choose on its own.
///
/// # Why a cursor and not a flattened list
///
/// The fill is parallel over rows and a row's operations can straddle a chunk boundary, which is
/// what a flattened `Vec<&BinaryInput>` was there to hide. Materializing it costs one pointer per
/// operation for the whole instance — 102 MB on a full `BinaryAddHiHuge`, which measured as 93% of
/// that air's witness time over a 4381-block run, the fill itself being the other 7%. A task
/// instead seeks a cursor to the first operation of its row range and walks forward from there
/// ([`InputCursor`], the same one [`fill_slots_and_tally`] uses), collecting one row's operations
/// at a time into a buffer of `inputs_per_row` pointers allocated once per task and reused.
///
/// # Panics
/// Panics if `rows` does not hold exactly one row per `inputs_per_row` operations, or if the chunks
/// hold a different number of operations than `total_inputs` announces. A mismatch would silently
/// drop rows (leaving the trace underfilled) or operations (losing them), neither of which surfaces
/// until the bus fails to balance. The checks are a couple of comparisons per call, not per row, so
/// they are worth keeping in release builds too.
pub fn fill_and_tally_chunked<R, Fill>(
    rows: &mut [R],
    inputs: &[Vec<BinaryInput>],
    total_inputs: usize,
    inputs_per_row: usize,
    fill: Fill,
) -> Vec<u32>
where
    R: Send,
    Fill: Fn(&mut R, &[&BinaryInput], &mut [u32]) + Sync + Send,
{
    assert!(inputs_per_row > 0, "a row must take at least one input");
    assert_eq!(
        rows.len(),
        total_inputs.div_ceil(inputs_per_row),
        "the rows must hold exactly the {total_inputs} inputs, {inputs_per_row} to a row",
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

    rows.par_chunks_mut(rows_per_task)
        .enumerate()
        .map(|(task, row_chunk)| {
            let mut multiplicities = vec![0u32; RANGE_16_BITS];
            // `rows_per_task` rows of `inputs_per_row` operations each, so the task's first
            // operation is at this global index — the same split the zipped slices used to make.
            let mut done = task * rows_per_task * inputs_per_row;
            let mut cursor = InputCursor::new(inputs, &starts, done);
            let mut row_inputs: Vec<&BinaryInput> = Vec::with_capacity(inputs_per_row);

            for row in row_chunk.iter_mut() {
                let filled = inputs_per_row.min(total_inputs - done);
                row_inputs.clear();
                for _ in 0..filled {
                    row_inputs
                        .push(cursor.next().expect("the cursor holds one input per filled slot"));
                }
                fill(row, &row_inputs, &mut multiplicities);
                done += filled;
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

/// Fills `rows` in parallel from the chunked `inputs`, tallying the table rows the fill looks up.
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
/// [`SparseTally`] histograms is bounded by the thread count.
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
) -> SparseTally
where
    R: Send,
    Slot: Fn(&mut R, usize, &BinaryInput, &mut SparseTally) + Sync + Send,
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
            let mut tally = SparseTally::new(table_rows, lookups_x_task);
            let mut done = task * rows_per_task * lanes_x_row;
            let mut cursor = InputCursor::new(inputs, &starts, done);

            for row in row_chunk.iter_mut() {
                let filled = lanes_x_row.min(total_inputs - done);
                for lane in 0..filled {
                    let input = cursor.next().expect("the cursor holds one input per filled slot");
                    slot(row, lane, input, &mut tally);
                }
                for lane in filled..lanes_x_row {
                    pad(row, lane);
                }
                done += filled;
            }
            tally
        })
        .reduce_with(SparseTally::merge)
        .unwrap_or_else(|| SparseTally::new(table_rows, lookups_x_task))
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
                    tally.counts(),
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

    /// The tally must match a plain serial histogram of the same chunks, whatever the split, and the
    /// rows must see the operations in global order — which is what the cursor replaced the
    /// flattened list to preserve. `chunked` puts each operation's global index in `a`, so the fill
    /// can assert the order rather than only the count.
    #[test]
    fn the_tally_matches_a_serial_count() {
        for inputs_per_row in [1usize, 3, 5] {
            for lengths in CHUNKINGS {
                let inputs = chunked(lengths);
                let total: usize = lengths.iter().sum();
                let mut filled = vec![0u64; total.div_ceil(inputs_per_row)];

                let multiplicities = fill_and_tally_chunked(
                    &mut filled,
                    &inputs,
                    total,
                    inputs_per_row,
                    |row, row_inputs, m| {
                        *row = row_inputs.len() as u64;
                        for input in row_inputs {
                            m[(input.a % 300) as usize] += 1;
                        }
                    },
                );

                let mut expected = vec![0u32; RANGE_16_BITS];
                for i in 0..total as u64 {
                    expected[(i % 300) as usize] += 1;
                }
                assert_eq!(multiplicities, expected, "{lengths:?}, {inputs_per_row} per row");

                // And every row was visited, with the operations that belong to it.
                assert_eq!(filled.iter().sum::<u64>(), total as u64, "{lengths:?}");
            }
        }
    }

    /// Every row must get exactly the operations at its own offsets, in order. A cursor that
    /// mis-seeks when a task's first row starts mid-chunk would still tally the right totals, so the
    /// histogram test above cannot catch it on its own.
    #[test]
    fn every_row_gets_its_own_operations_in_order() {
        for inputs_per_row in [1usize, 2, 3, 8] {
            for lengths in CHUNKINGS {
                let inputs = chunked(lengths);
                let total: usize = lengths.iter().sum();
                let mut seen = vec![Vec::new(); total.div_ceil(inputs_per_row)];

                fill_and_tally_chunked(
                    &mut seen,
                    &inputs,
                    total,
                    inputs_per_row,
                    |row, row_inputs, _| *row = row_inputs.iter().map(|i| i.a).collect::<Vec<_>>(),
                );

                let flat: Vec<u64> = seen.concat();
                assert_eq!(
                    flat,
                    (0..total as u64).collect::<Vec<_>>(),
                    "{lengths:?}, {inputs_per_row} per row: the rows did not see every operation \
                     exactly once, in order",
                );
                for (r, row) in seen.iter().enumerate() {
                    let want = inputs_per_row.min(total - r * inputs_per_row);
                    assert_eq!(row.len(), want, "{lengths:?}, {inputs_per_row} per row: row {r}");
                }
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
        // Ten operations three to a row need four rows, not three.
        fill_and_tally_chunked(&mut [0u64; 3], &chunked(&[4, 6]), 10, 3, |_, _, _| {});
    }

    /// The announced total and the chunks must agree, or the cursor and the row count would be
    /// walking different lists.
    #[test]
    #[should_panic(expected = "the chunks hold")]
    fn a_wrong_total_is_an_error() {
        fill_and_tally_chunked(&mut [0u64; 4], &chunked(&[4, 6]), 11, 3, |_, _, _| {});
    }

    /// No work means an all-zero tally rather than a panic on the empty reduction.
    #[test]
    fn nothing_to_fill_tallies_nothing() {
        let multiplicities =
            fill_and_tally_chunked(&mut [0u64; 0], &chunked(&[0, 0]), 0, 4, |_, _, m| m[1] += 1);
        assert_eq!(multiplicities.len(), RANGE_16_BITS);
        assert!(multiplicities.iter().all(|&m| m == 0));
    }
}
