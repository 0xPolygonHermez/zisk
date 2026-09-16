//! Parallel fills for the binary airs' trace rows.
//!
//! These used to tally the table multiplicities as they filled -- one histogram per rayon task,
//! merged and handed to `std` at the end -- because incrementing `std`'s shared array once per
//! *lookup* (`BinaryBasic` looks one up per byte: 134M contended atomics on a full `BinaryHuge`)
//! cost more than the fill itself. The prover now derives those multiplicities from the committed
//! trace, so the counting is gone and what is left is the filling:
//!
//! * [`fill_rows`] -- a plain parallel fill for the add airs, one row per group of inputs.
//! * [`fill_slots`] -- a fill for the airs that pack several operations into a row. It walks the
//!   chunked inputs with a cursor rather than flattening them, so no `Vec<&BinaryInput>` the size
//!   of the instance is built to be read once.

use crate::BinaryInput;
use rayon::prelude::*;

/// Fills `rows` in parallel, giving each row its slice of `inputs`.
///
/// `fill` receives one row and the `inputs_per_row` inputs that belong to it. The last row may get
/// a shorter slice when `inputs` does not divide evenly.
///
/// The rows are split into no more chunks than there are rayon threads.
///
/// # Panics
/// Panics if `rows` does not hold exactly one row per `inputs_per_row` inputs. The two sides are
/// zipped, so a mismatch would silently drop rows (leaving the trace underfilled) or inputs (losing
/// operations), neither of which surfaces until the bus fails to balance. The check is a couple of
/// comparisons per call, not per row, so it is worth keeping in release builds too.
pub fn fill_rows<R, T, Fill>(rows: &mut [R], inputs: &[T], inputs_per_row: usize, fill: Fill)
where
    R: Send,
    T: Sync,
    Fill: Fn(&mut R, &[T]) + Sync + Send,
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
        .for_each(|(row_chunk, input_chunk)| {
            for (row, row_inputs) in row_chunk.iter_mut().zip(input_chunk.chunks(inputs_per_row)) {
                fill(row, row_inputs);
            }
        });
}

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

/// Fills `rows` in parallel from the chunked `inputs`.
///
/// Slots are numbered consecutively across the whole instance, `lanes_x_row` to a row, so row `r`
/// takes operations `r * lanes_x_row ..`. `slot` fills one of them;
/// `pad` fills a slot of the last row that has no operation left — only the last row can be short,
/// and the trace buffer comes from a pool and is not zeroed, so those slots cannot be left alone.
///
/// The rows are split into no more chunks than there are rayon threads.
///
/// # Panics
/// Panics if `rows` does not hold exactly the `total_inputs` operations at `lanes_x_row` to a row.
pub fn fill_slots<R, Slot, Pad>(
    rows: &mut [R],
    inputs: &[Vec<BinaryInput>],
    total_inputs: usize,
    lanes_x_row: usize,
    slot: Slot,
    pad: Pad,
) where
    R: Send,
    Slot: Fn(&mut R, usize, &BinaryInput) + Sync + Send,
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

    rows.par_chunks_mut(rows_per_task)
        .enumerate()
        .for_each(|(task, row_chunk)| {
            let mut done = task * rows_per_task * lanes_x_row;
            let mut cursor = InputCursor::new(inputs, &starts, done);

            for row in row_chunk.iter_mut() {
                let filled = lanes_x_row.min(total_inputs - done);
                for lane in 0..filled {
                    let input = cursor.next().expect("the cursor holds one input per filled slot");
                    slot(row, lane, input);
                }
                for lane in filled..lanes_x_row {
                    pad(row, lane);
                }
                done += filled;
            }
        });
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

                fill_slots(
                    &mut rows,
                    &inputs,
                    total,
                    lanes_x_row,
                    |row, lane, input| row[lane] = input.a,
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

    /// Every row must be visited exactly once, with the inputs that belong to it, whatever the
    /// split: a row the fill skips is an underfilled trace that only surfaces as a bus imbalance.
    #[test]
    fn every_row_is_filled_once() {
        for inputs_per_row in [1usize, 3, 5] {
            for count in [0usize, 1, 7, 1000] {
                let inputs: Vec<u64> = (0..count as u64).map(|i| (i * 7) % 300).collect();
                let rows = count.div_ceil(inputs_per_row);
                let mut filled = vec![0u64; rows];

                fill_rows(&mut filled, &inputs, inputs_per_row, |row, row_inputs| {
                    *row = row_inputs.len() as u64;
                });

                assert_eq!(
                    filled.iter().sum::<u64>(),
                    count as u64,
                    "{count} inputs, {inputs_per_row} per row"
                );
            }
        }
    }

    /// The split never asks for more chunks than there are threads. It can be fewer — there is no
    /// work to give every thread when the rows are few.
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
        fill_rows(&mut [0u64; 3], &[0u64; 10], 3, |_, _| {});
    }

    /// No work is not an error: the empty split must simply do nothing.
    #[test]
    fn nothing_to_fill_is_not_an_error() {
        fill_rows(&mut [0u64; 0], &[0u64; 0], 4, |_, _| unreachable!("no rows to fill"));
    }
}
