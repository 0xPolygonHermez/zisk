//! The `BinaryAddSM` module implements the logic for the Binary Add State Machine.
//!
//! This state machine proves full 64-bit additions and, on the lanes that support it, the SH3ADD
//! operations whose shifted operand is a clean 32-bit value (see [`crate::sh3add_shape`]).
//!
//! Several operations are packed per row. The three airs differ in how many — `lanes_x_row` in
//! `binary_add.pil` — so the packing width is not a constant here: it comes from the row type's own
//! [`BinaryAddRow::LANES_X_ROW`], read from the generated trace row itself.

use crate::{fill_and_tally, BinaryInput, BinaryLanes};
use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use std::sync::Arc;
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{
    BinaryAddAirValues, BinaryAddHugeAirValues, BinaryAddHugeTrace, BinaryAddHugeTraceRowOps,
    BinaryAddLargeAirValues, BinaryAddLargeTrace, BinaryAddLargeTraceRowOps, BinaryAddTrace,
    BinaryAddTraceRowOps,
};

const MASK_U32: u64 = 0x0000_0000_FFFF_FFFF;

/// Limbs an operation splits into, and therefore carries it chains.
pub const LIMBS_X_ADD: usize = 2;

/// 16-bit chunks of the result each operation range-checks: two per limb.
///
/// Named apart from `BinaryAddHi`'s own `CHUNKS_X_FULL_ADD`, which is 2: that air materializes only the
/// low limb, so a name shared between the two would be a trap for anything importing both.
pub const CHUNKS_X_FULL_ADD: usize = LIMBS_X_ADD * 2;

/// Ties an add row type to the trace of the air it fills and to that air's packing width.
///
/// The three airs commit the same columns at different widths (`a[1]` vs `a[3]` vs `a[6]`), so they
/// cannot share a row type: each has its own, and `Self::LANES_X_ROW` is what tells the shared fill
/// logic how many slots to write.
pub trait BinaryAddRow<F: PrimeField64, T>: Default + Copy + Send + Sync {
    /// Operations this air packs into one row.
    const LANES_X_ROW: usize;

    /// Writes one slot of the row.
    ///
    /// `sh3add` selects the shifted addition; the air only has a `sel_sh3add` column for it because
    /// every lane is SH3ADD-capable (`sh3add_x_row` defaults to `lanes_x_row`), which is what lets
    /// this fill be a plain sequential walk with no restricted slots.
    fn set_slot(
        &mut self,
        lane: usize,
        a: &[u32; LIMBS_X_ADD],
        b: &[u32; LIMBS_X_ADD],
        c_chunks: &[u16; CHUNKS_X_FULL_ADD],
        cout: &[bool; LIMBS_X_ADD],
        sh3add: bool,
    );

    fn new_trace(trace_buffer: Vec<F>) -> ProofmanResult<T>;
    fn trace_num_rows(trace: &T) -> usize;
    fn trace_buffer_mut(trace: &mut T) -> &mut [Self];

    /// Fills the padding rows and wraps the trace into an `AirInstance`.
    ///
    /// `padding_size` is counted in *slots*, not rows: the bus sees one operation per slot, so what
    /// has to be cancelled is the number of empty slots.
    fn into_air_instance(trace: &mut T, rows_used: usize, padding_size: usize) -> AirInstance<F>;
}

/// Emits the row-to-trace binding for one add air. The bodies only differ in the widths the
/// generated setters take.
macro_rules! impl_binary_add_row {
    ($row_ops:ident, $trace:ident, $air_values:ident, $lanes:expr) => {
        impl<F: PrimeField64, R: $row_ops<F>> BinaryAddRow<F, $trace<R>> for R {
            const LANES_X_ROW: usize = $lanes;

            #[inline(always)]
            fn set_slot(
                &mut self,
                lane: usize,
                a: &[u32; LIMBS_X_ADD],
                b: &[u32; LIMBS_X_ADD],
                c_chunks: &[u16; CHUNKS_X_FULL_ADD],
                cout: &[bool; LIMBS_X_ADD],
                sh3add: bool,
            ) {
                for i in 0..LIMBS_X_ADD {
                    self.set_a(lane, i, a[i]);
                    self.set_b(lane, i, b[i]);
                    self.set_cout(lane, i, cout[i]);
                }
                for i in 0..CHUNKS_X_FULL_ADD {
                    self.set_c_chunks(lane, i, c_chunks[i]);
                }
                self.set_sel_sh3add(lane, sh3add);
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
                // Rows past the filled ones are all zeros: LANES_X_ROW additions of 0 + 0 = 0 each.
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

impl_binary_add_row!(
    BinaryAddTraceRowOps,
    BinaryAddTrace,
    BinaryAddAirValues,
    crate::lanes_x_row::ADD
);
impl_binary_add_row!(
    BinaryAddLargeTraceRowOps,
    BinaryAddLargeTrace,
    BinaryAddLargeAirValues,
    crate::lanes_x_row::ADD_LARGE
);
impl_binary_add_row!(
    BinaryAddHugeTraceRowOps,
    BinaryAddHugeTrace,
    BinaryAddHugeAirValues,
    crate::lanes_x_row::ADD_HUGE
);

/// The `BinaryAddSM` struct encapsulates the logic of the Binary Add State Machine.
pub struct BinaryAddSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,
    range_id: usize,
}

impl<F: PrimeField64> BinaryAddSM<F> {
    /// Creates a new BinaryAdd State Machine instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let range_id = std.get_range_id(0, 0xFFFF, None).expect("Failed to get range ID");

        Arc::new(Self { std, range_id })
    }

    /// Fills one slot of a row from one operation, and returns the chunks it range-checks.
    ///
    /// The shifted operand of an SH3ADD is folded into the addition rather than materialized: the
    /// bits `a << 3` carries out of a limb and the addition carry land on the same place, so the
    /// same carry chain transports both. `sh3add_shape` has already guaranteed that the shifted
    /// operand is a clean 32-bit value and that the low limb carries at most once, which is what
    /// keeps every `cout` a bit.
    #[inline(always)]
    pub fn process_slice<T, R: BinaryAddRow<F, T>>(
        &self,
        row: &mut R,
        lane: usize,
        input: &BinaryInput,
    ) -> [u64; CHUNKS_X_FULL_ADD] {
        let sh3add = input.op == ZiskOp::Sh3add.code();

        // SH3ADD is c = b + (a << 3), so the multiplier rides on the operand that gets shifted.
        // Its high limb is zero by construction (see `sh3add_shape`), which is why multiplying the
        // limbs separately is the same as shifting the whole 64-bit value.
        let scale = if sh3add { 8u64 } else { 1u64 };
        let a = input.a;
        let b = input.b;
        let mut cin = 0u64;

        let mut a_values = [0u32; LIMBS_X_ADD];
        let mut b_values = [0u32; LIMBS_X_ADD];
        let mut c_chunks_values = [0u16; CHUNKS_X_FULL_ADD];
        let mut cout_values = [false; LIMBS_X_ADD];
        let mut range_checks = [0u64; CHUNKS_X_FULL_ADD];

        for i in 0..LIMBS_X_ADD {
            // Extract the appropriate 32-bit chunk for this iteration
            let _a = if i == 0 { a & MASK_U32 } else { a >> 32 };
            let _b = if i == 0 { b & MASK_U32 } else { b >> 32 };
            let c = scale * _a + _b + cin;
            let _c = c & MASK_U32;

            // The air forces the shifted operand to be a clean 32-bit value, so every limb above
            // the first must be zero — `sh3add_shape` only routes such operations here. Writing a
            // non-zero one would break `sel_sh3add[lane] * a[lane][i] === 0` in the PIL, which is a
            // far harder failure to read than this.
            debug_assert!(
                !sh3add || i == 0 || _a == 0,
                "BinaryAdd: SH3ADD with a non-zero limb {i} of a ({:#x}); sh3add_shape should have \
                 kept this operation out",
                input.a,
            );

            // The columns carry the operands as the bus sees them: unshifted.
            a_values[i] = _a as u32;
            b_values[i] = _b as u32;

            // Split result into two 16-bit chunks (indices: i=0 -> 0,1; i=1 -> 2,3)
            c_chunks_values[i * 2] = (_c & 0xFFFF) as u16;
            c_chunks_values[i * 2 + 1] = (_c >> 16) as u16;

            // Update carry for next iteration
            cin = c >> 32;
            debug_assert!(
                cin <= 1,
                "BinaryAdd: carry {cin} out of limb {i} does not fit in a bit \
                 (op={:#x} a={a:#x} b={b:#x}); sh3add_shape should have kept this operation out",
                input.op,
            );
            cout_values[i] = cin != 0;

            range_checks[i * 2] = c_chunks_values[i * 2] as u64;
            range_checks[i * 2 + 1] = c_chunks_values[i * 2 + 1] as u64;
        }

        row.set_slot(lane, &a_values, &b_values, &c_chunks_values, &cout_values, sh3add);

        range_checks
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// Operations are written slot by slot, in order: every lane accepts every operation this air
    /// proves, so the fill is a plain sequential walk and the last row is the only partial one.
    pub fn compute_witness<T, R: BinaryAddRow<F, T>>(
        &self,
        inputs: &[Vec<BinaryInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let lanes = BinaryLanes::new(R::LANES_X_ROW);
        let mut add_trace = R::new_trace(trace_buffer)?;

        let num_rows = R::trace_num_rows(&add_trace);
        let num_slots = lanes.slots(num_rows);

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        debug_assert!(
            total_inputs <= num_slots,
            "BinaryAdd: {total_inputs} operations do not fit in {num_slots} slots",
        );

        tracing::debug!(
            "··· Creating BinaryAdd instance [{} / {} slots filled {:.2}%]",
            total_inputs,
            num_slots,
            total_inputs as f64 / num_slots as f64 * 100.0
        );

        let flat_inputs: Vec<_> = inputs.iter().flatten().collect();

        // Rows are filled LANES_X_ROW operations at a time, and each operation's chunks are tallied
        // as its slot is written rather than kept to be counted afterwards — see [`fill_and_tally`].
        let rows_used = lanes.rows_for(total_inputs);
        let mut multiplicities = fill_and_tally(
            &mut R::trace_buffer_mut(&mut add_trace)[..rows_used],
            &flat_inputs,
            R::LANES_X_ROW,
            |trace_row, row_inputs, multiplicities| {
                for (lane, input) in row_inputs.iter().enumerate() {
                    for chunk in self.process_slice::<T, R>(trace_row, lane, input) {
                        multiplicities[chunk as usize] += 1;
                    }
                }
                // Only the last row can be short. Its leftover lanes are not covered by the padding
                // rows written afterwards, so they are zeroed here: 0 + 0 = 0, the padding operation.
                // The trace buffer comes from a pool and is not zeroed, so leaving them would put
                // stale values on the bus.
                for lane in row_inputs.len()..R::LANES_X_ROW {
                    trace_row.set_slot(
                        lane,
                        &[0; LIMBS_X_ADD],
                        &[0; LIMBS_X_ADD],
                        &[0; CHUNKS_X_FULL_ADD],
                        &[false; LIMBS_X_ADD],
                        false,
                    );
                }
            },
        );

        // Every slot range-checks CHUNKS_X_FULL_ADD chunks, and an empty one is 0 + 0 = 0, so its chunks
        // are all zero: the slots left over on the last filled row included.
        let padding_size = num_slots - total_inputs;
        multiplicities[0] += (CHUNKS_X_FULL_ADD * padding_size) as u32;
        debug_assert_eq!(
            multiplicities.iter().map(|&m| m as u64).sum::<u64>(),
            CHUNKS_X_FULL_ADD as u64 * num_slots as u64,
            "the multiplicities must account for the chunks of every slot",
        );

        self.std.range_check_ranged(self.range_id, None, &multiplicities);

        Ok(R::into_air_instance(&mut add_trace, rows_used, padding_size))
    }
}
