//! The `BinaryAddHiSM` module implements the logic for the Binary Add Hi State Machine.
//!
//! This state machine proves the additions whose result fits in the low 32-bit limb, packing several
//! of them per row. Two operand shapes are supported (see [`crate::add_shape`]) and every slot takes
//! either of them, because the shape is determined by the carry out of the low limb and each slot
//! carries its own `sel_b_hi_is_ff` selector.
//!
//! There are two such airs and they differ in how many additions a row holds — [`ADDS_X_ROW`] for
//! `BinaryAddHi`, [`ADDS_X_ROW_LARGE`] for `BinaryAddHiLarge` — so the packing width is not a
//! constant here: it comes from [`BinaryAddHiRow::ADDS_X_ROW`], the row type's own.

use crate::{fill_and_tally, MAX_ADDS_X_ROW};
use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use std::sync::Arc;
use zisk_pil::{
    BinaryAddHiAirValues, BinaryAddHiLargeAirValues, BinaryAddHiLargeTrace,
    BinaryAddHiLargeTraceRowOps, BinaryAddHiTrace, BinaryAddHiTraceRowOps,
};

const MASK_32: u64 = 0x0000_0000_FFFF_FFFF;

/// Number of 16-bit chunks of the result that each packed addition range-checks.
pub const CHUNKS_X_ADD: usize = 2;

/// Ties an add-hi row type to the trace of the air it fills and to that air's packing width.
///
/// The two airs commit the same columns at different widths (`a[3]` vs `a[5]`, …), so unlike the
/// extension family they cannot share a row type: each has its own, and `Self::ADDS_X_ROW` is what
/// tells the shared fill logic how many slots to write.
pub trait BinaryAddHiRow<F: PrimeField64, T>: Default + Copy + Send + Sync {
    /// Additions this air packs into one row. Never above [`MAX_ADDS_X_ROW`].
    const ADDS_X_ROW: usize;

    /// Writes the row's slots. Each slice holds exactly [`Self::ADDS_X_ROW`] entries.
    fn set_slots(&mut self, a: &[u32], b: &[u32], c_chunks: &[[u16; CHUNKS_X_ADD]], sel: &[bool]);

    fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<T>;
    fn trace_num_rows(trace: &T) -> usize;
    fn trace_buffer_mut(trace: &mut T) -> &mut [Self];

    /// Fills the padding rows and wraps the trace into an `AirInstance`.
    ///
    /// `padding_size` is counted in *slots*, not rows: the bus sees one operation per slot, so what
    /// has to be cancelled is the number of empty slots.
    fn into_air_instance(trace: &mut T, rows_used: usize, padding_size: usize) -> AirInstance<F>;
}

/// Emits the row-to-trace binding for one add-hi air. The bodies only differ in the widths the
/// generated setters take, which is what the slice-to-array conversions pin.
macro_rules! impl_binary_add_hi_row {
    ($row_ops:ident, $trace:ident, $air_values:ident, $adds:expr) => {
        impl<F: PrimeField64, R: $row_ops<F>> BinaryAddHiRow<F, $trace<R>> for R {
            const ADDS_X_ROW: usize = $adds;

            #[inline(always)]
            fn set_slots(
                &mut self,
                a: &[u32],
                b: &[u32],
                c_chunks: &[[u16; CHUNKS_X_ADD]],
                sel: &[bool],
            ) {
                self.set_all_a(a.try_into().expect("a must hold ADDS_X_ROW slots"));
                self.set_all_b(b.try_into().expect("b must hold ADDS_X_ROW slots"));
                self.set_all_c_chunks(
                    c_chunks.try_into().expect("c_chunks must hold ADDS_X_ROW slots"),
                );
                self.set_all_sel_b_hi_is_ff(
                    sel.try_into().expect("sel must hold ADDS_X_ROW slots"),
                );
            }

            fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<$trace<R>> {
                $trace::<R>::new_from_vec(trace_buffer)
            }

            fn trace_num_rows(trace: &$trace<R>) -> usize {
                trace.num_rows()
            }

            fn trace_buffer_mut(trace: &mut $trace<R>) -> &mut [Self] {
                &mut trace.buffer
            }

            fn into_air_instance(
                trace: &mut $trace<R>,
                rows_used: usize,
                padding_size: usize,
            ) -> AirInstance<F> {
                // Rows past the packed ones are all zeros: ADDS_X_ROW additions of 0 + 0 = 0 each.
                let num_rows = trace.num_rows();
                if rows_used < num_rows {
                    let padding_row = R::default();
                    trace.buffer[rows_used..num_rows]
                        .par_iter_mut()
                        .for_each(|slot| *slot = padding_row);
                }

                let mut air_values = $air_values::<F>::new();
                air_values.padding_size = F::from_usize(padding_size);
                AirInstance::new_from_trace(FromTrace::new(trace).with_air_values(&mut air_values))
            }
        }
    };
}

impl_binary_add_hi_row!(
    BinaryAddHiTraceRowOps,
    BinaryAddHiTrace,
    BinaryAddHiAirValues,
    crate::ADDS_X_ROW
);
impl_binary_add_hi_row!(
    BinaryAddHiLargeTraceRowOps,
    BinaryAddHiLargeTrace,
    BinaryAddHiLargeAirValues,
    crate::ADDS_X_ROW_LARGE
);

/// Number of rows needed to pack `num_ops` additions into an air holding `adds_x_row` per row.
#[inline]
pub fn rows_needed(num_ops: u64, adds_x_row: usize) -> u64 {
    num_ops.div_ceil(adds_x_row as u64)
}

/// Operations one instance can hold, i.e. its capacity measured in operations.
#[inline]
pub fn ops_per_instance(num_rows: u64, adds_x_row: usize) -> u64 {
    adds_x_row as u64 * num_rows
}

/// The `BinaryAddHiSM` struct encapsulates the logic of the Binary Add Hi State Machine.
pub struct BinaryAddHiSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,
    range_id: usize,
}

impl<F: PrimeField64> BinaryAddHiSM<F> {
    /// Creates a new BinaryAddHi State Machine instance.
    ///
    /// # Arguments
    /// * `std` - An `Arc`-wrapped reference to the PIL2 standard library.
    ///
    /// # Returns
    /// A new `BinaryAddHiSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let range_id = std.get_range_id(0, 0xFFFF, None).expect("Failed to get range ID");

        Arc::new(Self { std, range_id })
    }

    /// Fills one slot of a row and returns the two 16-bit chunks of the result.
    ///
    /// The carry out of the low limb tells the two shapes apart, so it is also the selector: when it
    /// is set, `b` is a sign-extended negative value and the carry is what cancels its
    /// `0xFFFF_FFFF` high limb, leaving a zero high limb in the result.
    #[inline(always)]
    fn process_slot(
        input: &[u64; 2],
        a_values: &mut [u32; MAX_ADDS_X_ROW],
        b_values: &mut [u32; MAX_ADDS_X_ROW],
        c_chunks_values: &mut [[u16; CHUNKS_X_ADD]; MAX_ADDS_X_ROW],
        sel_values: &mut [bool; MAX_ADDS_X_ROW],
        slot: usize,
    ) -> [u64; CHUNKS_X_ADD] {
        let a = input[0] & MASK_32;
        let b = input[1] & MASK_32;

        let sum = a + b;
        let c = sum & MASK_32;

        a_values[slot] = a as u32;
        b_values[slot] = b as u32;
        c_chunks_values[slot][0] = (c & 0xFFFF) as u16;
        c_chunks_values[slot][1] = (c >> 16) as u16;
        sel_values[slot] = sum > MASK_32;

        [c_chunks_values[slot][0] as u64, c_chunks_values[slot][1] as u64]
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `inputs` - Per-chunk lists of additions, each as its two 64-bit bus operands.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness<T, R: BinaryAddHiRow<F, T>>(
        &self,
        inputs: &[Vec<[u64; 2]>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let adds_x_row = R::ADDS_X_ROW;
        debug_assert!(adds_x_row <= MAX_ADDS_X_ROW);

        let mut add_trace = R::new_trace(trace_buffer)?;
        let num_rows = R::trace_num_rows(&add_trace);

        // Flatten the per-chunk lists; operation i goes to slot i % adds_x_row of row i / adds_x_row.
        let flat_inputs: Vec<&[u64; 2]> = inputs.iter().flatten().collect();
        let total_inputs = flat_inputs.len();

        let rows_used = rows_needed(total_inputs as u64, adds_x_row) as usize;
        debug_assert!(rows_used <= num_rows, "{} <= {}", rows_used, num_rows);

        tracing::debug!(
            "··· Creating BinaryAddHi instance [{} ops in {} / {} rows filled {:.2}%]",
            total_inputs,
            rows_used,
            num_rows,
            rows_used as f64 / num_rows as f64 * 100.0
        );

        // The chunks of every slot are tallied as the row is filled — see [`fill_and_tally`]. An
        // empty slot in the last row proves the 0 + 0 = 0 addition, whose chunks are zero, so the
        // slots this row does not fill are counted with the padding below rather than here.
        let chunks_x_row = CHUNKS_X_ADD * adds_x_row;
        let mut multiplicities = fill_and_tally(
            &mut R::trace_buffer_mut(&mut add_trace)[..rows_used],
            &flat_inputs,
            adds_x_row,
            |trace_row, row_inputs, multiplicities| {
                let mut a_values = [0u32; MAX_ADDS_X_ROW];
                let mut b_values = [0u32; MAX_ADDS_X_ROW];
                let mut c_chunks_values = [[0u16; CHUNKS_X_ADD]; MAX_ADDS_X_ROW];
                let mut sel_values = [false; MAX_ADDS_X_ROW];

                for (slot, input) in row_inputs.iter().enumerate() {
                    let chunks = Self::process_slot(
                        input,
                        &mut a_values,
                        &mut b_values,
                        &mut c_chunks_values,
                        &mut sel_values,
                        slot,
                    );
                    multiplicities[chunks[0] as usize] += 1;
                    multiplicities[chunks[1] as usize] += 1;
                }

                trace_row.set_slots(
                    &a_values[..adds_x_row],
                    &b_values[..adds_x_row],
                    &c_chunks_values[..adds_x_row],
                    &sel_values[..adds_x_row],
                );
            },
        );

        // Every row range-checks all its slots' chunks unconditionally, and every chunk the fill did
        // not tally is a zero: the empty slots of the last packed row, and every slot of the rows
        // past it.
        multiplicities[0] += (chunks_x_row * num_rows - CHUNKS_X_ADD * total_inputs) as u32;
        debug_assert_eq!(
            multiplicities.iter().map(|&m| m as u64).sum::<u64>(),
            (chunks_x_row * num_rows) as u64,
            "the multiplicities must account for one chunk of every slot of every row",
        );

        self.std.range_check_ranged(self.range_id, None, &multiplicities);

        // The bus sees one operation per slot, so what has to be cancelled is the number of empty
        // slots, not the number of empty rows.
        let padding_size = adds_x_row * num_rows - total_inputs;
        Ok(R::into_air_instance(&mut add_trace, rows_used, padding_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ADDS_X_ROW, ADDS_X_ROW_LARGE};

    #[test]
    fn rows_needed_packs_ops_per_row() {
        for adds_x_row in [ADDS_X_ROW, ADDS_X_ROW_LARGE] {
            assert_eq!(rows_needed(0, adds_x_row), 0);
            assert_eq!(rows_needed(1, adds_x_row), 1);
            assert_eq!(rows_needed(adds_x_row as u64, adds_x_row), 1);
            assert_eq!(rows_needed(adds_x_row as u64 + 1, adds_x_row), 2);
        }
    }

    /// The row count must always leave room for every operation, and never waste a whole row.
    #[test]
    fn rows_needed_is_tight() {
        for adds_x_row in [ADDS_X_ROW, ADDS_X_ROW_LARGE] {
            for num_ops in 0..100u64 {
                let rows = rows_needed(num_ops, adds_x_row);
                assert!(rows * adds_x_row as u64 >= num_ops);
                assert!(rows == 0 || ((rows - 1) * adds_x_row as u64) < num_ops);
            }
        }
    }

    #[test]
    fn ops_per_instance_matches_the_packing() {
        for adds_x_row in [ADDS_X_ROW, ADDS_X_ROW_LARGE] {
            assert_eq!(rows_needed(ops_per_instance(10, adds_x_row), adds_x_row), 10);
        }
    }

    /// The per-row buffers are sized for the widest air, so the narrow one must fit in them.
    #[test]
    fn the_row_buffers_hold_the_widest_packing() {
        const {
            assert!(ADDS_X_ROW <= MAX_ADDS_X_ROW);
            assert!(ADDS_X_ROW_LARGE <= MAX_ADDS_X_ROW);
        }
    }
}
