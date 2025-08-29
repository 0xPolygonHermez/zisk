//! The `ArithFullSM` module implements the Arithmetic Full State Machine.
//!
//! This state machine manages the computation of arithmetic operations and their associated
//! trace generation. It coordinates with `ArithTableSM` and `ArithRangeTableSM` to handle
//! state transitions and multiplicity updates.

use std::sync::Arc;

use crate::{
    ArithOperation, ArithRangeTableHelpers, ArithRangeTableSM, ArithTableHelpers, ArithTableSM,
};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use rayon::prelude::*;
use sm_binary::{GT_OP, LTU_OP, LT_ABS_NP_OP, LT_ABS_PN_OP};
use zisk_common::{ExtOperationData, OperationBusData, OperationData, PayloadType};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
use zisk_pil::*;

const CHUNK_SIZE: u64 = 0x10000;
const EXTENSION: u64 = 0xFFFFFFFF;

/// The `ArithFullSM` struct represents the Arithmetic Full State Machine.
///
/// This state machine coordinates the computation of arithmetic operations and updates
/// the `ArithTableSM` and `ArithRangeTableSM` components based on operation traces.
pub struct ArithFullSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,
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
        Arc::new(Self { std })
    }

    /// Computes the witness for arithmetic operations and updates associated tables.
    ///
    /// # Arguments
    /// * `inputs` - A slice of `OperationData` representing the arithmetic inputs.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed arithmetic trace.
    pub fn compute_witness(
        &self,
        inputs: &[Vec<OperationData<u64>>],
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut arith_trace = ArithTrace::new_from_vec(trace_buffer);

        let num_rows = arith_trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        tracing::info!(
            "··· Creating Arith instance [{} / {} rows filled {:.2}%]",
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        // Split the arith_trace.buffer into slices matching each inner vector’s length.
        let flat_inputs: Vec<_> = inputs.iter().flatten().collect(); // Vec<&OperationData<u64>>
        let flat_buffer = arith_trace.row_slice_mut();
        let chunk_size = total_inputs.div_ceil(rayon::current_num_threads());

        flat_buffer.par_chunks_mut(chunk_size).zip(flat_inputs.par_chunks(chunk_size)).for_each(
            |(trace_slice, input_slice)| {
                trace_slice.iter_mut().zip(input_slice.iter()).for_each(|(trace_row, input)| {
                    *trace_row = Self::process_slice(input);
                });
            },
        );

        let padding_offset = total_inputs;
        let padding_rows: usize = num_rows.saturating_sub(padding_offset);

        if padding_rows > 0 {
            let mut t: ArithTraceRow<F> = Default::default();
            let padding_opcode = ZiskOp::Muluh.code();
            t.op = F::from_u8(padding_opcode);
            t.fab = F::ONE;

            arith_trace.row_slice_mut()[padding_offset..num_rows]
                .par_iter_mut()
                .for_each(|elem| *elem = t);
        }

        AirInstance::new_from_trace(FromTrace::new(&mut arith_trace))
    }

    pub fn compute_multiplicity_instance(&self, total_inputs: usize) {
        // Get the Arithmetic table ID
        let table_id = self.std.get_virtual_table_id(ArithTableSM::TABLE_ID);

        // Get the Arithmetic Range table ID
        let range_table_id = self.std.get_virtual_table_id(ArithRangeTableSM::TABLE_ID);

        let num_rows = ArithTrace::<usize>::NUM_ROWS;
        let padding_offset = total_inputs;
        let padding_rows: usize = num_rows.saturating_sub(padding_offset);

        if padding_rows > 0 {
            let padding_opcode = ZiskOp::Muluh.code();
            self.std.inc_virtual_row(
                range_table_id,
                ArithRangeTableHelpers::get_row_chunk_range_check(0, 0) as u64,
                padding_rows as u64 * 10,
            );
            self.std.inc_virtual_row(
                range_table_id,
                ArithRangeTableHelpers::get_row_chunk_range_check(26, 0) as u64,
                padding_rows as u64 * 2,
            );
            self.std.inc_virtual_row(
                range_table_id,
                ArithRangeTableHelpers::get_row_chunk_range_check(17, 0) as u64,
                padding_rows as u64 * 2,
            );
            self.std.inc_virtual_row(
                range_table_id,
                ArithRangeTableHelpers::get_row_chunk_range_check(9, 0) as u64,
                padding_rows as u64 * 2,
            );
            self.std.inc_virtual_row(
                range_table_id,
                ArithRangeTableHelpers::get_row_carry_range_check(0) as u64,
                padding_rows as u64 * 7,
            );
            self.std.inc_virtual_row(
                table_id,
                ArithTableHelpers::direct_get_row(
                    padding_opcode,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                    false,
                ) as u64,
                padding_rows as u64,
            );
        }
    }

    /// Generates binary inputs for operations requiring additional validation (e.g., division).
    #[inline(always)]
    pub fn generate_inputs(input: &OperationData<u64>) -> Vec<Vec<PayloadType>> {
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
            vec![OperationBusData::from_values(
                opcode,
                ZiskOperationType::Binary as u64,
                aop.d[0]
                    + CHUNK_SIZE * aop.d[1]
                    + CHUNK_SIZE.pow(2) * (aop.d[2] + extension.0)
                    + CHUNK_SIZE.pow(3) * aop.d[3],
                aop.b[0]
                    + CHUNK_SIZE * aop.b[1]
                    + CHUNK_SIZE.pow(2) * (aop.b[2] + extension.1)
                    + CHUNK_SIZE.pow(3) * aop.b[3],
            )
            .to_vec()]
        } else {
            vec![]
        }
    }

    #[inline(always)]
    pub fn process_multiplicity(std: &Std<F>, input: &[u64; 4]) {
        // Get the Arithmetic table ID
        let table_id = std.get_virtual_table_id(ArithTableSM::TABLE_ID);

        // Get the Arithmetic Range table ID
        let range_table_id = std.get_virtual_table_id(ArithRangeTableSM::TABLE_ID);

        let mut aop = ArithOperation::new();

        let input_data = ExtOperationData::OperationData(*input);

        let opcode = OperationBusData::get_op(&input_data);
        let a = OperationBusData::get_a(&input_data);
        let b = OperationBusData::get_b(&input_data);

        aop.calculate(opcode, a, b);
        for i in [0, 2] {
            std.inc_virtual_row(
                range_table_id,
                ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.a[i]) as u64,
                1,
            );
            std.inc_virtual_row(
                range_table_id,
                ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.b[i]) as u64,
                1,
            );
            std.inc_virtual_row(
                range_table_id,
                ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.c[i]) as u64,
                1,
            );
            std.inc_virtual_row(
                range_table_id,
                ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.d[i]) as u64,
                1,
            );
        }
        std.inc_virtual_row(
            range_table_id,
            ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_ab, aop.a[3]) as u64,
            1,
        );
        std.inc_virtual_row(
            range_table_id,
            ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_ab + 26, aop.a[1]) as u64,
            1,
        );
        std.inc_virtual_row(
            range_table_id,
            ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_ab + 17, aop.b[3]) as u64,
            1,
        );
        std.inc_virtual_row(
            range_table_id,
            ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_ab + 9, aop.b[1]) as u64,
            1,
        );

        std.inc_virtual_row(
            range_table_id,
            ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_cd, aop.c[3]) as u64,
            1,
        );
        std.inc_virtual_row(
            range_table_id,
            ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_cd + 26, aop.c[1]) as u64,
            1,
        );
        std.inc_virtual_row(
            range_table_id,
            ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_cd + 17, aop.d[3]) as u64,
            1,
        );
        std.inc_virtual_row(
            range_table_id,
            ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_cd + 9, aop.d[1]) as u64,
            1,
        );

        for i in 0..7 {
            std.inc_virtual_row(
                range_table_id,
                ArithRangeTableHelpers::get_row_carry_range_check(aop.carry[i]) as u64,
                1,
            );
        }

        let row = ArithTableHelpers::direct_get_row(
            aop.op,
            aop.na,
            aop.nb,
            aop.np,
            aop.nr,
            aop.sext,
            aop.div_by_zero,
            aop.div_overflow,
        );
        std.inc_virtual_row(table_id, row as u64, 1);
    }

    fn process_slice(input: &[u64; 4]) -> ArithTraceRow<F> {
        let mut aop = ArithOperation::new();
        let input_data = ExtOperationData::OperationData(*input);

        let opcode = OperationBusData::get_op(&input_data);
        let a = OperationBusData::get_a(&input_data);
        let b = OperationBusData::get_b(&input_data);

        aop.calculate(opcode, a, b);
        let mut t: ArithTraceRow<F> = Default::default();
        for i in [0, 2] {
            t.a[i] = F::from_u64(aop.a[i]);
            t.b[i] = F::from_u64(aop.b[i]);
            t.c[i] = F::from_u64(aop.c[i]);
            t.d[i] = F::from_u64(aop.d[i]);
        }
        for i in [1, 3] {
            t.a[i] = F::from_u64(aop.a[i]);
            t.b[i] = F::from_u64(aop.b[i]);
            t.c[i] = F::from_u64(aop.c[i]);
            t.d[i] = F::from_u64(aop.d[i]);
        }

        for i in 0..7 {
            t.carry[i] = F::from_i64(aop.carry[i]);
        }
        t.op = F::from_u8(aop.op);
        t.m32 = F::from_bool(aop.m32);
        t.div = F::from_bool(aop.div);
        t.na = F::from_bool(aop.na);
        t.nb = F::from_bool(aop.nb);
        t.np = F::from_bool(aop.np);
        t.nr = F::from_bool(aop.nr);
        t.signed = F::from_bool(aop.signed);
        t.main_mul = F::from_bool(aop.main_mul);
        t.main_div = F::from_bool(aop.main_div);
        t.sext = F::from_bool(aop.sext);
        t.multiplicity = F::ONE;
        t.range_ab = F::from_u8(aop.range_ab);
        t.range_cd = F::from_u8(aop.range_cd);
        t.div_by_zero = F::from_bool(aop.div_by_zero);
        t.div_overflow = F::from_bool(aop.div_overflow);
        t.inv_sum_all_bs = if aop.div && !aop.div_by_zero {
            F::from_u64(aop.b[0] + aop.b[1] + aop.b[2] + aop.b[3]).inverse()
        } else {
            F::ZERO
        };

        t.fab = if aop.na != aop.nb { F::NEG_ONE } else { F::ONE };
        //  na * (1 - 2 * nb);
        t.na_fb = if aop.na {
            if aop.nb {
                F::NEG_ONE
            } else {
                F::ONE
            }
        } else {
            F::ZERO
        };
        t.nb_fa = if aop.nb {
            if aop.na {
                F::NEG_ONE
            } else {
                F::ONE
            }
        } else {
            F::ZERO
        };
        t.bus_res1 = F::from_u64(if aop.sext {
            0xFFFFFFFF
        } else if aop.m32 {
            0
        } else if aop.main_mul {
            aop.c[2] + (aop.c[3] << 16)
        } else if aop.main_div {
            aop.a[2] + (aop.a[3] << 16)
        } else {
            aop.d[2] + (aop.d[3] << 16)
        });

        t
    }
}
