//! The `BinaryAddHiSM` module implements the logic for the Binary Add Hi State Machine.
//!
//! This state machine proves the additions whose result fits in the low 32-bit limb, packing several
//! of them per row. Two operand shapes are supported (see [`crate::add_shape`]) and every slot takes
//! either of them, because the shape is determined by the carry out of the low limb and each slot
//! carries its own `sel_b_hi_is_ff` selector.
//!
//! The three such airs differ in how many additions a row holds (`lanes_x_row` in
//! `binary_add_hi.pil`), so the packing width is not a constant here: it comes from
//! [`BinaryAddHiRow::LANES_X_ROW`], the row type's own. On top of the additions, every slot also
//! proves the SH3ADD operations of the same shape, selected by its own `sel_sh3add`.

use crate::{fill_rows, lanes_x_row::MAX_ADD_HI, BinaryInput};
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use std::sync::Arc;
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{
    BinaryAddHiAirValues, BinaryAddHiHugeAirValues, BinaryAddHiHugeTrace,
    BinaryAddHiLargeAirValues, BinaryAddHiLargeTrace, BinaryAddHiTrace,
};
use zisk_pil::{
    BinaryAddHiHugeTraceRow, BinaryAddHiHugeTraceRowPacked, BinaryAddHiLargeTraceRow,
    BinaryAddHiLargeTraceRowPacked, BinaryAddHiTraceRow, BinaryAddHiTraceRowPacked,
};

const MASK_32: u64 = 0x0000_0000_FFFF_FFFF;

/// Number of 16-bit chunks of the result that each packed addition range-checks.
pub const CHUNKS_X_ADD: usize = 2;

/// Ties an add-hi row type to the trace of the air it fills and to that air's packing width.
///
/// The two airs commit the same columns at different widths (`a[3]` vs `a[5]`, …), so unlike the
/// extension family they cannot share a row type: each has its own, and `Self::LANES_X_ROW` is what
/// tells the shared fill logic how many slots to write.
/// The columns of a `BinaryAddHi` lane, as its fill writes them, on whichever row holds them: a row
/// of one of the three add-hi airs, or the `add_hi_` block of the fused `CompactBinary` row (see
/// `compact_binary_rows.rs`).
pub trait BinaryAddHiLaneRow<F: PrimeField64>: Default + Copy + Send + Sync {
    /// Additions this air packs into one row. Never above [`MAX_ADD_HI`].
    const LANES_X_ROW: usize;

    /// Writes the row's slots. Each slice holds exactly [`Self::LANES_X_ROW`] entries.
    ///
    /// `sh3add` selects the shifted addition on a slot. Every slot is SH3ADD-capable
    /// (`sh3add_x_row` defaults to `lanes_x_row` in the PIL), so any operation goes in any slot.
    fn set_slots(
        &mut self,
        a: &[u32],
        b: &[u32],
        c_chunks: &[[u16; CHUNKS_X_ADD]],
        sel: &[bool],
        sh3add: &[bool],
    );

    /// Overwrites this block of the row with `src`'s, leaving every other block untouched. The
    /// padding of an instance is one row repeated, built once and copied: on a standalone row the
    /// block IS the row, on a fused row the copy must not take the other blocks with it.
    #[inline(always)]
    fn copy_block_from(&mut self, src: &Self) {
        *self = *src;
    }
}

/// Ties an add-hi row type to the trace of the air it fills.
pub trait BinaryAddHiRow<F: PrimeField64, T>: BinaryAddHiLaneRow<F> {
    fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<T>;
    fn trace_num_rows(trace: &T) -> usize;
    fn trace_buffer_mut(trace: &mut T) -> &mut [Self];

    /// Wraps the filled trace into an `AirInstance`.
    ///
    /// `padding_size` is counted in *slots*, not rows: the bus sees one operation per slot, so what
    /// has to be cancelled is the number of empty slots.
    fn into_air_instance(trace: &mut T, padding_size: usize) -> AirInstance<F>;
}

/// Emits the row-to-trace binding for one add-hi air. The bodies only differ in the widths the
/// generated setters take, which is what the slice-to-array conversions pin.
macro_rules! impl_binary_add_hi_row {
    ($row_ops:ident, $trace:ident, $air_values:ident, $adds:expr, [$($row:ident),+]) => {
        $(
        impl<F: PrimeField64> BinaryAddHiLaneRow<F> for $row<F> {
            const LANES_X_ROW: usize = $adds;

            #[inline(always)]
            fn set_slots(
                &mut self,
                a: &[u32],
                b: &[u32],
                c_chunks: &[[u16; CHUNKS_X_ADD]],
                sel: &[bool],
                sh3add: &[bool],
            ) {
                self.set_all_a(a.try_into().expect("a must hold LANES_X_ROW slots"));
                self.set_all_b(b.try_into().expect("b must hold LANES_X_ROW slots"));
                self.set_all_c_chunks(
                    c_chunks.try_into().expect("c_chunks must hold LANES_X_ROW slots"),
                );
                self.set_all_sel_b_hi_is_ff(
                    sel.try_into().expect("sel must hold LANES_X_ROW slots"),
                );
                self.set_all_sel_sh3add(
                    sh3add.try_into().expect("sh3add must hold LANES_X_ROW slots"),
                );
            }
        }
        )+

        $(
        impl<F: PrimeField64> BinaryAddHiRow<F, $trace<$row<F>>> for $row<F> {
            fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<$trace<$row<F>>> {
                $trace::<$row<F>>::new_from_vec(trace_buffer)
            }

            fn trace_num_rows(trace: &$trace<$row<F>>) -> usize {
                trace.num_rows()
            }

            fn trace_buffer_mut(trace: &mut $trace<$row<F>>) -> &mut [Self] {
                &mut trace.buffer
            }

            fn into_air_instance(trace: &mut $trace<$row<F>>, padding_size: usize) -> AirInstance<F> {
                let mut air_values = $air_values::<F>::new();
                air_values.padding_size = F::from_usize(padding_size);
                AirInstance::new_from_trace(FromTrace::new(trace).with_air_values(&mut air_values))
            }
        }
        )+
    };
}

impl_binary_add_hi_row!(
    BinaryAddHiTraceRowOps,
    BinaryAddHiTrace,
    BinaryAddHiAirValues,
    crate::lanes_x_row::ADD_HI,
    [BinaryAddHiTraceRow, BinaryAddHiTraceRowPacked]
);
impl_binary_add_hi_row!(
    BinaryAddHiLargeTraceRowOps,
    BinaryAddHiLargeTrace,
    BinaryAddHiLargeAirValues,
    crate::lanes_x_row::ADD_HI_LARGE,
    [BinaryAddHiLargeTraceRow, BinaryAddHiLargeTraceRowPacked]
);
impl_binary_add_hi_row!(
    BinaryAddHiHugeTraceRowOps,
    BinaryAddHiHugeTrace,
    BinaryAddHiHugeAirValues,
    crate::lanes_x_row::ADD_HI_HUGE,
    [BinaryAddHiHugeTraceRow, BinaryAddHiHugeTraceRowPacked]
);

/// Number of rows needed to pack `num_ops` additions into an air holding `lanes_x_row` per row.
#[inline]
pub fn rows_needed(num_ops: u64, lanes_x_row: usize) -> u64 {
    num_ops.div_ceil(lanes_x_row as u64)
}

/// Operations one instance can hold, i.e. its capacity measured in operations.
#[inline]
pub fn ops_per_instance(num_rows: u64, lanes_x_row: usize) -> u64 {
    lanes_x_row as u64 * num_rows
}

/// The `BinaryAddHiSM` struct encapsulates the logic of the Binary Add Hi State Machine.
pub struct BinaryAddHiSM<F: PrimeField64> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> BinaryAddHiSM<F> {
    /// Takes no `Std`: the chunks this air range-checks are counted by the prover from the trace.
    pub fn new() -> Arc<Self> {
        Arc::new(Self { _phantom: std::marker::PhantomData })
    }

    /// Fills one slot of a row and returns the two 16-bit chunks of the result.
    ///
    /// The carry out of the low limb tells the two shapes apart, so it is also the selector: when it
    /// is set, `b` is a sign-extended negative value and the carry is what cancels its
    /// `0xFFFF_FFFF` high limb, leaving a zero high limb in the result.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    fn process_slot(
        input: &BinaryInput,
        a_values: &mut [u32; MAX_ADD_HI],
        b_values: &mut [u32; MAX_ADD_HI],
        c_chunks_values: &mut [[u16; CHUNKS_X_ADD]; MAX_ADD_HI],
        sel_values: &mut [bool; MAX_ADD_HI],
        sh3add_values: &mut [bool; MAX_ADD_HI],
        slot: usize,
    ) {
        let sh3add = input.op == ZiskOp::Sh3add.code();

        // SH3ADD is c = b + (a << 3). The shift is folded into the addition rather than
        // materialized: the high limb of `a` is zero by construction (see `crate::sh3add_shape`),
        // so scaling the low limb is the same as shifting the whole 64-bit value.
        let scale = if sh3add { 8u64 } else { 1u64 };
        let a = input.a & MASK_32;
        let b = input.b & MASK_32;

        let sum = scale * a + b;
        let c = sum & MASK_32;

        // The shapes are told apart by the carry out of the low limb, which is what cancels the
        // 0xFFFF_FFFF high limb of a negative `b`. Both classifiers only route operations whose
        // carry is at most one, which is what makes the selector a bit.
        let carry = sum >> 32;
        debug_assert!(
            carry <= 1,
            "BinaryAddHi: carry {carry} out of the low limb does not fit in a bit \
             (op={:#x} a={:#x} b={:#x}); the shape classifiers should have kept this out",
            input.op,
            input.a,
            input.b,
        );

        // The columns carry the operands as the bus sees them: unshifted.
        a_values[slot] = a as u32;
        b_values[slot] = b as u32;
        c_chunks_values[slot][0] = (c & 0xFFFF) as u16;
        c_chunks_values[slot][1] = (c >> 16) as u16;
        sel_values[slot] = carry != 0;
        sh3add_values[slot] = sh3add;
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
        inputs: &[Vec<BinaryInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut add_trace = R::new_trace(trace_buffer)?;
        let padding_size = self.fill_rows(R::trace_buffer_mut(&mut add_trace), inputs);
        Ok(R::into_air_instance(&mut add_trace, padding_size))
    }

    /// Fills `rows` -- a whole instance, or the `add_hi_` block of one -- with `inputs` in order
    /// and pads the rest, returning the padding in slots. Takes `rows` rather than a trace because
    /// the fused `CompactBinary` air carries this block inside a wider row (see
    /// [`BinaryAddHiLaneRow`]).
    pub(crate) fn fill_rows<R: BinaryAddHiLaneRow<F>>(
        &self,
        rows: &mut [R],
        inputs: &[Vec<BinaryInput>],
    ) -> usize {
        let lanes_x_row = R::LANES_X_ROW;
        debug_assert!(lanes_x_row <= MAX_ADD_HI);

        let num_rows = rows.len();

        // Flatten the per-chunk lists; operation i goes to slot i % lanes_x_row of row i / lanes_x_row.
        let __t = std::time::Instant::now();
        let mut flat_inputs: Vec<&BinaryInput> =
            Vec::with_capacity(inputs.iter().map(|v| v.len()).sum());
        flat_inputs.extend(inputs.iter().flatten());
        let _report = crate::FlattenReport {
            name: "BinaryAddHi",
            inputs: flat_inputs.len(),
            flatten: __t.elapsed(),
            started: std::time::Instant::now(),
        };
        let total_inputs = flat_inputs.len();

        let rows_used = rows_needed(total_inputs as u64, lanes_x_row) as usize;
        debug_assert!(rows_used <= num_rows, "{} <= {}", rows_used, num_rows);

        tracing::debug!(
            "··· Creating BinaryAddHi instance [{} ops in {} / {} rows filled {:.2}%]",
            total_inputs,
            rows_used,
            num_rows,
            rows_used as f64 / num_rows as f64 * 100.0
        );

        fill_rows(&mut rows[..rows_used], &flat_inputs, lanes_x_row, |trace_row, row_inputs| {
            let mut a_values = [0u32; MAX_ADD_HI];
            let mut b_values = [0u32; MAX_ADD_HI];
            let mut c_chunks_values = [[0u16; CHUNKS_X_ADD]; MAX_ADD_HI];
            let mut sel_values = [false; MAX_ADD_HI];
            let mut sh3add_values = [false; MAX_ADD_HI];

            for (slot, input) in row_inputs.iter().enumerate() {
                Self::process_slot(
                    input,
                    &mut a_values,
                    &mut b_values,
                    &mut c_chunks_values,
                    &mut sel_values,
                    &mut sh3add_values,
                    slot,
                );
            }

            trace_row.set_slots(
                &a_values[..lanes_x_row],
                &b_values[..lanes_x_row],
                &c_chunks_values[..lanes_x_row],
                &sel_values[..lanes_x_row],
                &sh3add_values[..lanes_x_row],
            );
        });

        // Rows past the packed ones are all zeros: LANES_X_ROW additions of 0 + 0 = 0 each.
        if rows_used < num_rows {
            let padding_row = R::default();
            rows[rows_used..].par_iter_mut().for_each(|row| row.copy_block_from(&padding_row));
        }

        // The bus sees one operation per slot, so what has to be cancelled is the number of empty
        // slots, not the number of empty rows.
        lanes_x_row * num_rows - total_inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lanes_x_row::{ADD_HI, ADD_HI_HUGE, ADD_HI_LARGE};

    #[test]
    fn rows_needed_packs_ops_per_row() {
        for lanes_x_row in [ADD_HI, ADD_HI_LARGE, ADD_HI_HUGE] {
            assert_eq!(rows_needed(0, lanes_x_row), 0);
            assert_eq!(rows_needed(1, lanes_x_row), 1);
            assert_eq!(rows_needed(lanes_x_row as u64, lanes_x_row), 1);
            assert_eq!(rows_needed(lanes_x_row as u64 + 1, lanes_x_row), 2);
        }
    }

    /// The row count must always leave room for every operation, and never waste a whole row.
    #[test]
    fn rows_needed_is_tight() {
        for lanes_x_row in [ADD_HI, ADD_HI_LARGE, ADD_HI_HUGE] {
            for num_ops in 0..100u64 {
                let rows = rows_needed(num_ops, lanes_x_row);
                assert!(rows * lanes_x_row as u64 >= num_ops);
                assert!(rows == 0 || ((rows - 1) * lanes_x_row as u64) < num_ops);
            }
        }
    }

    #[test]
    fn ops_per_instance_matches_the_packing() {
        for lanes_x_row in [ADD_HI, ADD_HI_LARGE, ADD_HI_HUGE] {
            assert_eq!(rows_needed(ops_per_instance(10, lanes_x_row), lanes_x_row), 10);
        }
    }

    /// The per-row buffers are sized for the widest air, so the narrow one must fit in them.
    #[test]
    fn the_row_buffers_hold_the_widest_packing() {
        const {
            assert!(ADD_HI <= MAX_ADD_HI);
            assert!(ADD_HI_LARGE <= MAX_ADD_HI);
            assert!(ADD_HI_HUGE <= MAX_ADD_HI, "the fixed-size row buffers must hold every air");
        }
    }
}
