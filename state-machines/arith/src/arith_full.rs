//! The `ArithFullSM` module implements the Arithmetic Full State Machine.
//!
//! This state machine manages the computation of arithmetic operations and their associated
//! trace generation. It coordinates with `ArithTableSM` and `ArithRangeTableSM` to handle
//! state transitions and multiplicity updates.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::{
    ArithFrops, ArithOperation, ArithRangeTableInputs, ArithRangeTableSM, ArithTableInputs,
    ArithTableSM,
};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use rayon::prelude::*;
use sm_binary::{GT_OP, LTU_OP, LT_ABS_NP_OP, LT_ABS_PN_OP};
use zisk_common::{BusId, ExtOperationData, OperationBusData, OperationData};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
#[cfg(not(feature = "packed"))]
use zisk_pil::{ArithTrace, ArithTraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{ArithTracePacked, ArithTraceRowPacked};

#[cfg(feature = "packed")]
type ArithTraceRowType<F> = ArithTraceRowPacked<F>;
#[cfg(feature = "packed")]
type ArithTraceType<F> = ArithTracePacked<F>;

#[cfg(not(feature = "packed"))]
type ArithTraceRowType<F> = ArithTraceRow<F>;
#[cfg(not(feature = "packed"))]
type ArithTraceType<F> = ArithTrace<F>;

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

    /// The table ID for the FROPS
    frops_table_id: usize,
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
        let table_id = std.get_virtual_table_id(ArithTableSM::TABLE_ID);

        // Get the Arithmetic Range table ID
        let range_table_id = std.get_virtual_table_id(ArithRangeTableSM::TABLE_ID);

        // Get the Arithmetic FROPS table ID
        let frops_table_id = std.get_virtual_table_id(ArithFrops::TABLE_ID);

        Arc::new(Self { std, table_id, range_table_id, frops_table_id })
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
        let mut arith_trace = ArithTraceType::new_from_vec(trace_buffer);

        let num_rows = arith_trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        let mut range_table_inputs = ArithRangeTableInputs::new();
        let mut table_inputs = ArithTableInputs::new();

        tracing::info!(
            "··· Creating Arith instance [{} / {} rows filled {:.2}%]",
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        // Split the arith_trace.buffer into slices matching each inner vector’s length.
        let flat_inputs: Vec<_> = inputs.iter().flatten().collect(); // Vec<&OperationData<u64>>
        let flat_buffer = arith_trace.buffer.as_mut_slice();
        let chunk_size = total_inputs.div_ceil(rayon::current_num_threads());

        flat_buffer.par_chunks_mut(chunk_size).zip(flat_inputs.par_chunks(chunk_size)).for_each(
            |(trace_slice, input_slice)| {
                let mut aop = ArithOperation::new();
                let mut range_table = ArithRangeTableInputs::new();
                let mut table = ArithTableInputs::new();

                trace_slice.iter_mut().zip(input_slice.iter()).for_each(|(trace_row, input)| {
                    *trace_row = Self::process_slice(&mut range_table, &mut table, &mut aop, input);
                });

                for (row, multiplicity) in &table {
                    self.std.inc_virtual_row(self.table_id, row as u64, multiplicity);
                }

                for (row, multiplicity) in &range_table {
                    self.std.inc_virtual_row(self.range_table_id, row as u64, multiplicity);
                }
            },
        );

        let padding_offset = total_inputs;
        let padding_rows: usize = num_rows.saturating_sub(padding_offset);

        if padding_rows > 0 {
            let mut row = ArithTraceRowType::<F>::default();
            let padding_opcode = ZiskOp::Muluh.code();
            row.set_op(padding_opcode);
            row.set_fab(1);

            arith_trace.buffer[padding_offset..num_rows]
                .par_iter_mut()
                .for_each(|elem| *elem = row);

            range_table_inputs.multi_use_chunk_range_check(padding_rows * 10, 0, 0);
            range_table_inputs.multi_use_chunk_range_check(padding_rows * 2, 26, 0);
            range_table_inputs.multi_use_chunk_range_check(padding_rows * 2, 17, 0);
            range_table_inputs.multi_use_chunk_range_check(padding_rows * 2, 9, 0);
            range_table_inputs.multi_use_carry_range_check(padding_rows * 7, 0);
            table_inputs.multi_add_use(
                padding_rows,
                padding_opcode,
                false,
                false,
                false,
                false,
                false,
                false,
                false,
            );
        }

        // TODO: We should compare against cache-then-increase version instead of increase each time...

        for (row, multiplicity) in &table_inputs {
            self.std.inc_virtual_row(self.table_id, row as u64, multiplicity);
        }

        for (row, multiplicity) in &range_table_inputs {
            self.std.inc_virtual_row(self.range_table_id, row as u64, multiplicity);
        }

        AirInstance::new_from_trace(FromTrace::new(&mut arith_trace))
    }

    pub fn compute_frops(&self, frops_inputs: &Vec<u32>) {
        for row in frops_inputs {
            self.std.inc_virtual_row(self.frops_table_id, *row as u64, 1);
        }
    }

    /// Generates binary inputs for operations requiring additional validation (e.g., division).
    #[inline(always)]
    pub fn generate_inputs(input: &OperationData<u64>, pending: &mut VecDeque<(BusId, Vec<u64>)>) {
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
                aop.d[0]
                    + CHUNK_SIZE * aop.d[1]
                    + CHUNK_SIZE.pow(2) * (aop.d[2] + extension.0)
                    + CHUNK_SIZE.pow(3) * aop.d[3],
                aop.b[0]
                    + CHUNK_SIZE * aop.b[1]
                    + CHUNK_SIZE.pow(2) * (aop.b[2] + extension.1)
                    + CHUNK_SIZE.pow(3) * aop.b[3],
                pending,
            );
        }
    }

    fn process_slice(
        range_table_inputs: &mut ArithRangeTableInputs,
        table_inputs: &mut ArithTableInputs,
        aop: &mut ArithOperation,
        input: &[u64; 4],
    ) -> ArithTraceRowType<F> {
        let input_data = ExtOperationData::OperationData(*input);

        let opcode = OperationBusData::get_op(&input_data);
        let a = OperationBusData::get_a(&input_data);
        let b = OperationBusData::get_b(&input_data);

        aop.calculate(opcode, a, b);
        let mut row = ArithTraceRowType::<F>::default();
        for i in [0, 2] {
            row.set_a(i, aop.a[i] as u16);
            row.set_b(i, aop.b[i] as u16);
            row.set_c(i, aop.c[i] as u16);
            row.set_d(i, aop.d[i] as u16);
            range_table_inputs.use_chunk_range_check(0, aop.a[i]);
            range_table_inputs.use_chunk_range_check(0, aop.b[i]);
            range_table_inputs.use_chunk_range_check(0, aop.c[i]);
            range_table_inputs.use_chunk_range_check(0, aop.d[i]);
        }
        for i in [1, 3] {
            row.set_a(i, aop.a[i] as u16);
            row.set_b(i, aop.b[i] as u16);
            row.set_c(i, aop.c[i] as u16);
            row.set_d(i, aop.d[i] as u16);
        }
        range_table_inputs.use_chunk_range_check(aop.range_ab, aop.a[3]);
        range_table_inputs.use_chunk_range_check(aop.range_ab + 26, aop.a[1]);
        range_table_inputs.use_chunk_range_check(aop.range_ab + 17, aop.b[3]);
        range_table_inputs.use_chunk_range_check(aop.range_ab + 9, aop.b[1]);

        range_table_inputs.use_chunk_range_check(aop.range_cd, aop.c[3]);
        range_table_inputs.use_chunk_range_check(aop.range_cd + 26, aop.c[1]);
        range_table_inputs.use_chunk_range_check(aop.range_cd + 17, aop.d[3]);
        range_table_inputs.use_chunk_range_check(aop.range_cd + 9, aop.d[1]);

        for i in 0..7 {
            let carry = if aop.carry[i] >= 0 {
                aop.carry[i] as u64
            } else {
                (aop.carry[i] + F::ORDER_U64 as i64) as u64
            };
            row.set_carry(i, carry);
            range_table_inputs.use_carry_range_check(aop.carry[i]);
        }
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
        row.set_multiplicity(true);
        row.set_range_ab(aop.range_ab);
        row.set_range_cd(aop.range_cd);
        row.set_div_by_zero(aop.div_by_zero);
        row.set_div_overflow(aop.div_overflow);

        let inv_sum_all_bs = if aop.div && !aop.div_by_zero {
            F::from_u64(aop.b[0] + aop.b[1] + aop.b[2] + aop.b[3]).inverse().as_canonical_u64()
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
        );

        let fab = if aop.na != aop.nb { F::ORDER_U64 - 1 } else { 1 };
        row.set_fab(fab);

        let na_fb = if aop.na {
            if aop.nb {
                F::ORDER_U64 - 1
            } else {
                1
            }
        } else {
            0
        };
        //  na * (1 - 2 * nb);
        row.set_na_fb(na_fb);
        let nb_fa = if aop.nb {
            if aop.na {
                F::ORDER_U64 - 1
            } else {
                1
            }
        } else {
            0
        };
        row.set_nb_fa(nb_fa);

        let bus_res1 = if aop.sext {
            0xFFFFFFFF
        } else if aop.m32 {
            0
        } else if aop.main_mul {
            aop.c[2] + (aop.c[3] << 16)
        } else if aop.main_div {
            aop.a[2] + (aop.a[3] << 16)
        } else {
            aop.d[2] + (aop.d[3] << 16)
        };

        row.set_bus_res1(bus_res1 as u32);

        row
    }
}
