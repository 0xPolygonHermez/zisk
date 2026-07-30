//! The `ArithFullSM` module implements the Arithmetic Full State Machine.
//!
//! This state machine manages the computation of arithmetic operations and their associated
//! trace generation. It coordinates with `ArithTableSM` and `ArithRangeTableSM` to handle
//! state transitions and multiplicity updates.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::{
    ArithOperation, ArithRangeTableInputs, ArithRangeTableSM, ArithTableInputs, ArithTableSM,
    ARITH_RANGE_16_BITS,
};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use rayon::prelude::*;
use sm_binary::{GT_OP, LTU_OP, LT_ABS_NP_OP, LT_ABS_PN_OP};
use zisk_common::{BusId, ExtOperationData, OperationBusData, OperationData};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
use zisk_pil::{ArithAirValues, ArithTrace, ArithTraceRowOps};

const CHUNK_SIZE: u64 = 0x10000;
const EXTENSION: u64 = 0xFFFFFFFF;

/// The `ArithFullSM` struct represents the Arithmetic Full State Machine.
///
/// This state machine coordinates the computation of arithmetic operations and updates
/// the `ArithTableSM` and `ArithRangeTableSM` components based on operation traces.
pub struct ArithFullSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,

    /// The table ID for the Table State Machine
    table_id: usize,

    /// The table ID for the Range Table State Machine
    range_table_id: usize,
}

impl<F: PrimeField64> ArithFullSM<F> {
    /// Creates a new `ArithFullSM` instance.
    ///
    /// # Arguments
    /// * `std` - An `Arc`-wrapped reference to the PIL2 standard library.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithFullSM`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Get the Arithmetic table ID
        let table_id =
            std.get_virtual_table_id(ArithTableSM::TABLE_ID).expect("Failed to get table ID");

        // Get the Arithmetic Range table ID
        let range_table_id = std
            .get_virtual_table_id(ArithRangeTableSM::TABLE_ID)
            .expect("Failed to get range table ID");

        Arc::new(Self { std, table_id, range_table_id })
    }

    /// Computes the witness for arithmetic operations and updates associated tables.
    ///
    /// # Arguments
    /// * `inputs` - A slice of `OperationData` representing the arithmetic inputs.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed arithmetic trace.
    pub fn compute_witness<R: ArithTraceRowOps<F>>(
        &self,
        inputs: &[Vec<OperationData<u64>>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut arith_trace = ArithTrace::<R>::new_from_vec(trace_buffer)?;

        let num_rows = arith_trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        let mut range_table_inputs = ArithRangeTableInputs::new();
        let mut table_inputs = ArithTableInputs::new();

        tracing::debug!(
            "··· Creating Arith instance [{} / {} rows filled {:.2}%]",
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        // Split the arith_trace.buffer into slices matching each inner vector’s length.
        if total_inputs > 0 {
            let flat_inputs: Vec<_> = inputs.iter().flatten().collect(); // Vec<&OperationData<u64>>
            let flat_buffer = arith_trace.buffer.as_mut_slice();
            let chunk_size = total_inputs.div_ceil(rayon::current_num_threads());

            flat_buffer
                .par_chunks_mut(chunk_size)
                .zip(flat_inputs.par_chunks(chunk_size))
                .for_each(|(trace_slice, input_slice)| {
                    let mut aop = ArithOperation::new();
                    let mut range_table = ArithRangeTableInputs::new();
                    let mut table = ArithTableInputs::new();

                    trace_slice.iter_mut().zip(input_slice.iter()).for_each(
                        |(trace_row, input)| {
                            *trace_row = Self::process_slice::<R>(
                                &mut range_table,
                                &mut table,
                                &mut aop,
                                input,
                            );
                        },
                    );

                    for (row, multiplicity) in &table {
                        self.std.inc_virtual_row(self.table_id, row as u64, multiplicity);
                    }

                    for (row, multiplicity) in &range_table {
                        self.std.inc_virtual_row(self.range_table_id, row as u64, multiplicity);
                    }
                });
        }

        let padding_offset = total_inputs;
        let padding_rows: usize = num_rows.saturating_sub(padding_offset);

        if padding_rows > 0 {
            // `proves_operation` has no multiplicity selector, so every row of the trace proves an
            // operation on the bus, padding included. arith.pil cancels those with
            //   assumes_padding_operation(op: OP_MULU, a:[0,0], b:[0,0], c:[0,0], flag:0, padding_size:)
            // so the padding row must be exactly that trivial operation: mulu(0, 0) = (0, 0).
            //
            // The ArithTable and ArithRangeTable lookups have no selector either, so the padding row
            // also has to match a table entry. Derive it from a real ArithOperation rather than
            // hand-writing the columns: an all-zero row only worked while the all-FULL range id
            // happened to be 0, and hardcoded flags would have to be kept in sync with the table.
            let padding_opcode = ZiskOp::Mulu.code();
            let mut pad = ArithOperation::new();
            pad.calculate(padding_opcode, 0, 0);

            let mut row = R::default();
            row.set_op(padding_opcode);
            row.set_main_mul(pad.main_mul);
            row.set_result_is_zero(pad.result_is_zero);
            row.set_range_ab(pad.range_ab);
            row.set_range_cd(pad.range_cd);
            // Every other column of this operation is zero, which is what R::default() already gives:
            // a = b = c = d = 0, no sign flags, no division flags, and all carries zero.
            // `padding_row_tests` checks that this list is still complete.

            arith_trace.buffer[padding_offset..num_rows]
                .par_iter_mut()
                .for_each(|elem| *elem = row);

            // Range checks, mirroring what process_slice registers for a real row: the eight even
            // chunks go through the generic 16-bit range, and each of the four slots of range_ab and
            // range_cd is hit once.
            range_table_inputs.multi_use_chunk_range_check(
                padding_rows * 8,
                ARITH_RANGE_16_BITS,
                0,
            );
            for offset in 0..4 {
                range_table_inputs.multi_use_chunk_range_check(
                    padding_rows,
                    pad.range_ab + offset,
                    0,
                );
                range_table_inputs.multi_use_chunk_range_check(
                    padding_rows,
                    pad.range_cd + offset,
                    0,
                );
            }
            range_table_inputs.multi_use_carry_range_check(padding_rows * 7, 0);
            table_inputs.multi_add_use(
                padding_rows,
                padding_opcode,
                pad.na,
                pad.nb,
                pad.np,
                pad.nr,
                pad.sext,
                pad.div_by_zero,
                pad.div_overflow,
                pad.result_is_zero,
                pad.remainder_is_zero,
            );
        }

        // TODO: We should compare against cache-then-increase version instead of increase each time...

        for (row, multiplicity) in &table_inputs {
            self.std.inc_virtual_row(self.table_id, row as u64, multiplicity);
        }

        for (row, multiplicity) in &range_table_inputs {
            self.std.inc_virtual_row(self.range_table_id, row as u64, multiplicity);
        }

        // arith.pil uses this to cancel exactly `padding_rows` trivial operations off the bus.
        let mut air_values = ArithAirValues::<F>::new();
        air_values.padding_size = F::from_usize(padding_rows);

        Ok(AirInstance::new_from_trace(
            FromTrace::new(&mut arith_trace).with_air_values(&mut air_values),
        ))
    }

    /// Generates binary inputs for operations requiring additional validation (e.g., division).
    #[inline(always)]
    pub fn generate_inputs(
        input: &OperationData<u64>,
        pending: &mut VecDeque<(BusId, Vec<u64>, Vec<u64>)>,
    ) {
        let mut aop = ArithOperation::new();

        let input_data = ExtOperationData::OperationData(*input);

        let opcode = OperationBusData::get_op(&input_data);
        let a = OperationBusData::get_a(&input_data);
        let b = OperationBusData::get_b(&input_data);

        aop.calculate(opcode, a, b);

        // If the operation is a division, then use the binary component
        // to check that the remainer is lower than the divisor
        if aop.div && !aop.div_by_zero {
            let opcode = match (aop.nr, aop.nb) {
                (false, false) => LTU_OP,
                (false, true) => LT_ABS_PN_OP,
                (true, false) => LT_ABS_NP_OP,
                (true, true) => GT_OP,
            };

            let extension = match (aop.m32, aop.nr, aop.nb) {
                (false, _, _) => (0, 0),
                (true, false, false) => (0, 0),
                (true, false, true) => (0, EXTENSION),
                (true, true, false) => (EXTENSION, 0),
                (true, true, true) => (EXTENSION, EXTENSION),
            };

            // TODO: We dont need to "glue" the d,b chunks back, we can use the aop API to do this!
            OperationBusData::from_values(
                opcode,
                ZiskOperationType::Binary as u64,
                aop.d[0] as u64
                    + CHUNK_SIZE * aop.d[1] as u64
                    + CHUNK_SIZE.pow(2) * (aop.d[2] as u64 + extension.0)
                    + CHUNK_SIZE.pow(3) * aop.d[3] as u64,
                aop.b[0] as u64
                    + CHUNK_SIZE * aop.b[1] as u64
                    + CHUNK_SIZE.pow(2) * (aop.b[2] as u64 + extension.1)
                    + CHUNK_SIZE.pow(3) * aop.b[3] as u64,
                pending,
            );
        }
    }

    fn process_slice<R: ArithTraceRowOps<F>>(
        range_table_inputs: &mut ArithRangeTableInputs,
        table_inputs: &mut ArithTableInputs,
        aop: &mut ArithOperation,
        input: &[u64; 4],
    ) -> R {
        let input_data = ExtOperationData::OperationData(*input);

        let opcode = OperationBusData::get_op(&input_data);
        let a = OperationBusData::get_a(&input_data);
        let b = OperationBusData::get_b(&input_data);

        aop.calculate(opcode, a, b);
        let mut row = R::default();
        for i in [0, 2] {
            range_table_inputs.use_chunk_range_check(ARITH_RANGE_16_BITS, aop.a[i] as u64);
            range_table_inputs.use_chunk_range_check(ARITH_RANGE_16_BITS, aop.b[i] as u64);
            range_table_inputs.use_chunk_range_check(ARITH_RANGE_16_BITS, aop.c[i] as u64);
            range_table_inputs.use_chunk_range_check(ARITH_RANGE_16_BITS, aop.d[i] as u64);
        }
        row.set_all_a(&aop.a);
        row.set_all_b(&aop.b);
        row.set_all_c(&aop.c);
        row.set_all_d(&aop.d);
        // the four slots of a rid are consecutive: x3, x1, y3, y1
        range_table_inputs.use_chunk_range_check(aop.range_ab, aop.a[3] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_ab + 1, aop.a[1] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_ab + 2, aop.b[3] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_ab + 3, aop.b[1] as u64);

        range_table_inputs.use_chunk_range_check(aop.range_cd, aop.c[3] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_cd + 1, aop.c[1] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_cd + 2, aop.d[3] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_cd + 3, aop.d[1] as u64);

        let mut carry_values = [0u64; 7];
        for (i, carry_value) in carry_values.iter_mut().enumerate() {
            let carry = if aop.carry[i] >= 0 {
                aop.carry[i] as u64
            } else {
                (aop.carry[i] + F::ORDER_U64 as i64) as u64
            };
            *carry_value = carry;
            range_table_inputs.use_carry_range_check(aop.carry[i]);
        }
        row.set_all_carry(&carry_values);

        row.set_op(aop.op);
        row.set_m32(aop.m32);
        row.set_div(aop.div);
        row.set_na(aop.na);
        row.set_nb(aop.nb);
        row.set_np(aop.np);
        row.set_nr(aop.nr);
        row.set_signed(aop.signed);
        row.set_main_mul(aop.main_mul);
        row.set_main_div(aop.main_div);
        row.set_sext(aop.sext);
        row.set_range_ab(aop.range_ab);
        row.set_range_cd(aop.range_cd);
        row.set_div_by_zero(aop.div_by_zero);
        row.set_div_overflow(aop.div_overflow);
        row.set_result_is_zero(aop.result_is_zero);
        row.set_remainder_is_zero(aop.remainder_is_zero);

        let inv_sum_all_bs = if aop.div && !aop.div_by_zero {
            F::from_u64(aop.b[0] as u64 + aop.b[1] as u64 + aop.b[2] as u64 + aop.b[3] as u64)
                .inverse()
                .as_canonical_u64()
        } else {
            0
        };
        row.set_inv_sum_all_bs(inv_sum_all_bs);

        table_inputs.add_use(
            aop.op,
            aop.na,
            aop.nb,
            aop.np,
            aop.nr,
            aop.sext,
            aop.div_by_zero,
            aop.div_overflow,
            aop.result_is_zero,
            aop.remainder_is_zero,
        );

        row
    }
}
