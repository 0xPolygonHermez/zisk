use std::sync::Arc;

use crate::{
    ArithOperation, ArithRangeTableInputs, ArithRangeTableSM, ArithTableInputs, ArithTableSM,
};
use log::info;
use p3_field::PrimeField;
use proofman_common::{AirInstance, FromTrace};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use sm_binary::{GT_OP, LTU_OP, LT_ABS_NP_OP, LT_ABS_PN_OP};
use sm_common::i64_to_u64_field;
use zisk_core::{zisk_ops::ZiskOp, ZiskRequiredOperation};
use zisk_pil::*;

const CHUNK_SIZE: u64 = 0x10000;
const EXTENSION: u64 = 0xFFFFFFFF;

pub struct ArithFullSM {
    arith_table_sm: Arc<ArithTableSM>,
    arith_range_table_sm: Arc<ArithRangeTableSM>,
}

impl ArithFullSM {
    const MY_NAME: &'static str = "Arith   ";

    pub fn new(
        arith_table_sm: Arc<ArithTableSM>,
        arith_range_table_sm: Arc<ArithRangeTableSM>,
    ) -> Arc<Self> {
        Arc::new(Self { arith_table_sm, arith_range_table_sm })
    }

    pub fn prove_instance<F: PrimeField>(
        &self,
        inputs: &[ZiskRequiredOperation],
    ) -> AirInstance<F> {
        let mut range_table_inputs = ArithRangeTableInputs::new();
        let mut table_inputs = ArithTableInputs::new();

        let mut arith_trace = ArithTrace::new();

        let num_rows = arith_trace.num_rows();

        timer_start_trace!(ARITH_TRACE);
        info!(
            "{}: ··· Creating Arith instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            inputs.len(),
            num_rows,
            inputs.len() as f64 / num_rows as f64 * 100.0
        );
        assert!(inputs.len() <= num_rows);

        let mut aop = ArithOperation::new();
        for (irow, input) in inputs.iter().enumerate() {
            log::debug!(
                "#{} ARITH op:0x{:X} a:0x{:X} b:0x{:X}",
                irow,
                input.opcode,
                input.a,
                input.b
            );
            aop.calculate(input.opcode, input.a, input.b);
            let mut t: ArithTraceRow<F> = Default::default();
            for i in [0, 2] {
                t.a[i] = F::from_canonical_u64(aop.a[i]);
                t.b[i] = F::from_canonical_u64(aop.b[i]);
                t.c[i] = F::from_canonical_u64(aop.c[i]);
                t.d[i] = F::from_canonical_u64(aop.d[i]);
                range_table_inputs.use_chunk_range_check(0, aop.a[i]);
                range_table_inputs.use_chunk_range_check(0, aop.b[i]);
                range_table_inputs.use_chunk_range_check(0, aop.c[i]);
                range_table_inputs.use_chunk_range_check(0, aop.d[i]);
            }
            for i in [1, 3] {
                t.a[i] = F::from_canonical_u64(aop.a[i]);
                t.b[i] = F::from_canonical_u64(aop.b[i]);
                t.c[i] = F::from_canonical_u64(aop.c[i]);
                t.d[i] = F::from_canonical_u64(aop.d[i]);
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
                t.carry[i] = F::from_canonical_u64(i64_to_u64_field(aop.carry[i]));
                range_table_inputs.use_carry_range_check(aop.carry[i]);
            }
            t.op = F::from_canonical_u8(aop.op);
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
            t.multiplicity = F::one();
            t.debug_main_step = F::from_canonical_u64(input.step);
            t.range_ab = F::from_canonical_u8(aop.range_ab);
            t.range_cd = F::from_canonical_u8(aop.range_cd);
            t.div_by_zero = F::from_bool(aop.div_by_zero);
            t.div_overflow = F::from_bool(aop.div_overflow);
            t.inv_sum_all_bs = if aop.div && !aop.div_by_zero {
                F::from_canonical_u64(aop.b[0] + aop.b[1] + aop.b[2] + aop.b[3]).inverse()
            } else {
                F::zero()
            };

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

            t.fab = if aop.na != aop.nb { F::neg_one() } else { F::one() };
            //  na * (1 - 2 * nb);
            t.na_fb = if aop.na {
                if aop.nb {
                    F::neg_one()
                } else {
                    F::one()
                }
            } else {
                F::zero()
            };
            t.nb_fa = if aop.nb {
                if aop.na {
                    F::neg_one()
                } else {
                    F::one()
                }
            } else {
                F::zero()
            };
            t.bus_res1 = F::from_canonical_u64(if aop.sext {
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
            arith_trace[irow] = t;

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

                // TODO: We dont need to "glue" the d,b chunks back, we can use the aop API to do
                // this!
                let _operation = ZiskRequiredOperation {
                    step: input.step,
                    opcode,
                    a: aop.d[0] +
                        CHUNK_SIZE * aop.d[1] +
                        CHUNK_SIZE.pow(2) * (aop.d[2] + extension.0) +
                        CHUNK_SIZE.pow(3) * aop.d[3],
                    b: aop.b[0] +
                        CHUNK_SIZE * aop.b[1] +
                        CHUNK_SIZE.pow(2) * (aop.b[2] + extension.1) +
                        CHUNK_SIZE.pow(3) * aop.b[3],
                };
                // TODO!!!!!! Push to the DataBus
                // binary_inputs.push(operation);
            }
        }
        timer_stop_and_log_trace!(ARITH_TRACE);

        timer_start_trace!(ARITH_PADDING);
        let padding_offset = inputs.len();
        let padding_rows: usize = num_rows.saturating_sub(padding_offset);

        if padding_rows > 0 {
            let mut t: ArithTraceRow<F> = Default::default();
            let padding_opcode = ZiskOp::Muluh.code();
            t.op = F::from_canonical_u8(padding_opcode);
            t.fab = F::one();
            for i in padding_offset..num_rows {
                arith_trace[i] = t;
            }
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
        timer_stop_and_log_trace!(ARITH_PADDING);

        timer_start_trace!(ARITH_TABLE);
        self.arith_table_sm.process_slice(&table_inputs);
        timer_stop_and_log_trace!(ARITH_TABLE);

        timer_start_trace!(ARITH_RANGE_TABLE);
        self.arith_range_table_sm.process_slice(&range_table_inputs);
        timer_stop_and_log_trace!(ARITH_RANGE_TABLE);

        AirInstance::new_from_trace(FromTrace::new(&mut arith_trace))
    }
}
