//! The `BinaryAddHiSM` module implements the logic for the Binary Add Hi State Machine.
//!
//! This state machine proves the additions whose result fits in the low 32-bit limb, packing
//! [`ADDS_X_ROW`] of them per row. Two operand shapes are supported (see [`crate::add_shape`]) and
//! every slot takes either of them, because the shape is determined by the carry out of the low
//! limb and each slot carries its own `sel_b_hi_is_ff` selector.

use crate::ADDS_X_ROW;
use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use std::sync::Arc;
use zisk_pil::{BinaryAddHiAirValues, BinaryAddHiTrace, BinaryAddHiTraceRowOps};

const MASK_32: u64 = 0x0000_0000_FFFF_FFFF;

/// Number of 16-bit chunks of the result that each packed addition range-checks.
const CHUNKS_X_ADD: usize = 2;

/// Number of rows needed to pack `num_ops` additions.
#[inline]
pub fn rows_needed(num_ops: u64) -> u64 {
    num_ops.div_ceil(ADDS_X_ROW as u64)
}

/// Operations one instance can hold, i.e. its capacity measured in operations.
#[inline]
pub fn ops_per_instance(num_rows: u64) -> u64 {
    ADDS_X_ROW as u64 * num_rows
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
        a_values: &mut [u32; ADDS_X_ROW],
        b_values: &mut [u32; ADDS_X_ROW],
        c_chunks_values: &mut [[u16; CHUNKS_X_ADD]; ADDS_X_ROW],
        sel_values: &mut [bool; ADDS_X_ROW],
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
    pub fn compute_witness<R: BinaryAddHiTraceRowOps<F>>(
        &self,
        inputs: &[Vec<[u64; 2]>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut add_trace = BinaryAddHiTrace::<R>::new_from_vec(trace_buffer)?;

        let num_rows = add_trace.num_rows();

        // Flatten the per-chunk lists; operation i goes to slot i % ADDS_X_ROW of row
        // i / ADDS_X_ROW.
        let flat_inputs: Vec<&[u64; 2]> = inputs.iter().flatten().collect();
        let total_inputs = flat_inputs.len();

        let rows_used = rows_needed(total_inputs as u64) as usize;
        debug_assert!(rows_used <= num_rows, "{} <= {}", rows_used, num_rows);

        tracing::debug!(
            "··· Creating BinaryAddHi instance [{} ops in {} / {} rows filled {:.2}%]",
            total_inputs,
            rows_used,
            num_rows,
            rows_used as f64 / num_rows as f64 * 100.0
        );

        let mut range_checks: Vec<[u64; CHUNKS_X_ADD * ADDS_X_ROW]> =
            vec![[0u64; CHUNKS_X_ADD * ADDS_X_ROW]; rows_used];

        add_trace.buffer[..rows_used]
            .par_iter_mut()
            .zip(range_checks.par_iter_mut())
            .zip(flat_inputs.par_chunks(ADDS_X_ROW))
            .for_each(|((trace_row, range_check), row_inputs)| {
                let mut a_values = [0u32; ADDS_X_ROW];
                let mut b_values = [0u32; ADDS_X_ROW];
                let mut c_chunks_values = [[0u16; CHUNKS_X_ADD]; ADDS_X_ROW];
                let mut sel_values = [false; ADDS_X_ROW];

                // A slot left empty in the last row proves the 0 + 0 = 0 addition, which the
                // padding accounting below cancels on the bus.
                for (slot, input) in row_inputs.iter().enumerate() {
                    let chunks = Self::process_slot(
                        input,
                        &mut a_values,
                        &mut b_values,
                        &mut c_chunks_values,
                        &mut sel_values,
                        slot,
                    );
                    range_check[slot * CHUNKS_X_ADD] = chunks[0];
                    range_check[slot * CHUNKS_X_ADD + 1] = chunks[1];
                }

                trace_row.set_all_a(&a_values);
                trace_row.set_all_b(&b_values);
                trace_row.set_all_c_chunks(&c_chunks_values);
                trace_row.set_all_sel_b_hi_is_ff(&sel_values);
            });

        // Every row range-checks all its result chunks unconditionally. `range_checks` spans the
        // packed rows only, and already carries a zero for every empty slot inside them; the fully
        // padded rows are all zeros and are not covered there.
        let mut multiplicities = vec![0u32; 0xFFFF + 1];
        for range_check in range_checks {
            for chunk in range_check {
                multiplicities[chunk as usize] += 1;
            }
        }
        multiplicities[0] += (CHUNKS_X_ADD * ADDS_X_ROW * (num_rows - rows_used)) as u32;

        self.std.range_check_ranged(self.range_id, None, &multiplicities);

        // Rows past the packed ones are all zeros: ADDS_X_ROW additions of 0 + 0 = 0 each.
        if rows_used < num_rows {
            let padding_row = R::default();
            add_trace.buffer[rows_used..num_rows]
                .par_iter_mut()
                .for_each(|slot| *slot = padding_row);
        }

        // The bus sees one operation per slot, so what has to be cancelled is the number of empty
        // slots, not the number of empty rows.
        let mut air_values = BinaryAddHiAirValues::<F>::new();
        air_values.padding_size = F::from_usize(ADDS_X_ROW * num_rows - total_inputs);
        Ok(AirInstance::new_from_trace(
            FromTrace::new(&mut add_trace).with_air_values(&mut air_values),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_needed_packs_ops_per_row() {
        assert_eq!(rows_needed(0), 0);
        assert_eq!(rows_needed(1), 1);
        assert_eq!(rows_needed(ADDS_X_ROW as u64), 1);
        assert_eq!(rows_needed(ADDS_X_ROW as u64 + 1), 2);
    }

    /// The row count must always leave room for every operation, and never waste a whole row.
    #[test]
    fn rows_needed_is_tight() {
        for num_ops in 0..100u64 {
            let rows = rows_needed(num_ops);
            assert!(rows * ADDS_X_ROW as u64 >= num_ops);
            assert!(rows == 0 || ((rows - 1) * ADDS_X_ROW as u64) < num_ops);
        }
    }

    #[test]
    fn ops_per_instance_matches_the_packing() {
        assert_eq!(rows_needed(ops_per_instance(10)), 10);
    }
}
