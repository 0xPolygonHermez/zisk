//! Filling trace rows in parallel while tallying their range-checked chunks.
//!
//! The add airs range-check the 16-bit chunks of every result, so the witness has to hand `std` a
//! multiplicity per value of that range. The chunks themselves are of no further use: they are
//! written into the row and counted, and nothing reads them again.
//!
//! Keeping them to count afterwards is what the obvious shape does, and it scales with the *height*
//! of the instance — one entry per range check, so `2²³ × 4 × 8` bytes on a full `BinaryAddLarge`,
//! hundreds of megabytes of memory allocated, zeroed and touched to hold values read exactly once.
//!
//! Tallying as the rows are filled scales with the *number of tasks* instead: one histogram of
//! [`RANGE_16_BITS`] counters, 256 KB, per task, merged at the end. That is a few megabytes whatever
//! the air's height, and it costs no extra pass over the data.

use rayon::prelude::*;

/// Values a 16-bit range check can take, i.e. the width of one histogram.
pub const RANGE_16_BITS: usize = 0xFFFF + 1;

/// Fills `rows` in parallel, giving each row its slice of `inputs`, and returns the multiplicities
/// the fill tallied.
///
/// `fill` receives one row, the `inputs_per_row` inputs that belong to it, and the histogram of the
/// task it is running on — which it increments directly, one per range-checked chunk it produces.
/// The last row may get a shorter slice when `inputs` does not divide evenly.
///
/// The work is split into exactly one chunk per rayon thread, so the histograms are as few as they
/// can be while every thread still has one to itself.
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
    debug_assert!(inputs_per_row > 0, "a row must take at least one input");

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

#[cfg(test)]
mod tests {
    use super::*;

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

    /// No work means an all-zero tally rather than a panic on the empty reduction.
    #[test]
    fn nothing_to_fill_tallies_nothing() {
        let multiplicities = fill_and_tally(&mut [0u64; 0], &[0u64; 0], 4, |_, _, m| m[1] += 1);
        assert_eq!(multiplicities.len(), RANGE_16_BITS);
        assert!(multiplicities.iter().all(|&m| m == 0));
    }
}
