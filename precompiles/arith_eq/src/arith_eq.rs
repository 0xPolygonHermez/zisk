use std::sync::Arc;

use log::info;
use p3_field::PrimeField64;
use sm_common::i64_to_u64_field;

use proofman_common::{AirInstance, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::ArithEqTrace;

use crate::{
    arith_eq_constants::*, executors, Arith256Input, Arith256ModInput, ArithEqInput,
    Secp256k1AddInput, Secp256k1DblInput,
};

/// The `ArithEqSM` struct encapsulates the logic of the ArithEq State Machine.
pub struct ArithEqSM {
    /// Number of available arith256s in the trace.
    pub num_available_ops: usize,
}

impl ArithEqSM {
    const MY_NAME: &'static str = "ArithEq  ";

    /// Creates a new ArithEq State Machine instance.
    ///
    /// # Returns
    /// A new `ArithEqSM` instance.
    pub fn new() -> Arc<Self> {
        // Compute some useful values
        let num_available_ops = ArithEqTrace::<usize>::NUM_ROWS / ARITH_EQ_ROWS_BY_OP;

        Arc::new(Self { num_available_ops })
    }

    fn process_arith256<F: PrimeField64>(
        executor: &executors::Arith256,
        input: &Arith256Input,
        trace: &mut ArithEqTrace<F>,
        row_offset: usize,
    ) {
        let data = executor.execute(&input.a, &input.b, &input.c);
        Self::expand_data_on_trace(&data, row_offset, trace, 0);
    }

    fn process_arith256_mod<F: PrimeField64>(
        executor: &executors::Arith256Mod,
        input: &Arith256ModInput,
        trace: &mut ArithEqTrace<F>,
        row_offset: usize,
    ) {
        let data = executor.execute(&input.a, &input.b, &input.c, &input.module);
        Self::expand_data_on_trace(&data, row_offset, trace, 1);
    }
    fn process_secp256k1_add<F: PrimeField64>(
        executor: &executors::Secp256k1,
        input: &Secp256k1AddInput,
        trace: &mut ArithEqTrace<F>,
        row_offset: usize,
    ) {
        let data = executor.execute_add(&input.p1, &input.p2);
        Self::expand_data_on_trace(&data, row_offset, trace, 2);
    }
    fn process_secp256k1_dbl<F: PrimeField64>(
        executor: &executors::Secp256k1,
        input: &Secp256k1DblInput,
        trace: &mut ArithEqTrace<F>,
        row_offset: usize,
    ) {
        let data = executor.execute_dbl(&input.p1);
        Self::expand_data_on_trace(&data, row_offset, trace, 3);
    }

    fn expand_data_on_trace<F: PrimeField64>(
        data: &executors::ArithEqData,
        row_offset: usize,
        trace: &mut ArithEqTrace<F>,
        sel_op: usize,
    ) {
        for i in 0..ARITH_EQ_ROWS_BY_OP {
            let irow = row_offset + i;
            for j in 0..3 {
                trace[irow].carry[j][0] =
                    F::from_canonical_u64(i64_to_u64_field(data.cout[j][i * 2]));
                trace[irow].carry[j][1] =
                    F::from_canonical_u64(i64_to_u64_field(data.cout[j][i * 2 + 1]));
            }
            trace[irow].x1 = F::from_canonical_u16(data.x1[i] as u16);
            trace[irow].y1 = F::from_canonical_u16(data.y1[i] as u16);
            trace[irow].x2 = F::from_canonical_u16(data.x2[i] as u16);
            trace[irow].y2 = F::from_canonical_u16(data.y2[i] as u16);
            trace[irow].x3 = F::from_canonical_u16(data.x3[i] as u16);
            trace[irow].y3 = F::from_canonical_u16(data.y3[i] as u16);
            trace[irow].q0 = F::from_canonical_u64(i64_to_u64_field(data.q0[i]));
            trace[irow].q1 = F::from_canonical_u64(i64_to_u64_field(data.q1[i]));
            trace[irow].q2 = F::from_canonical_u64(i64_to_u64_field(data.q2[i]));
            trace[irow].s = F::from_canonical_u64(i64_to_u64_field(data.s[i]));
            for j in 0..4 {
                trace[irow].sel_op[j] = F::from_bool(j == sel_op);
            }
            // TODO:
            trace[irow].x_are_different = F::zero();
            trace[irow].x_delta_chunk_inv = F::zero();
            trace[irow].lt_borrow = F::zero();
            trace[irow].lt = F::zero();
            // if i == data.different_chunk {
            //     trace[irow].x_are_different = F::from_bool(true);
            //     trace[irow].x_delta_chunk_inv = data.delta_chunk_inv;
            // } else {
            //     trace[irow].x_are_different = F::zero();
            //     trace[irow].x_delta_chunk_inv = F::zero();
            // }
            // trace[irow].lt_borrow = F::from_bool(data.eq[0][i * 4] as u16);
            // trace[irow].lt = F::from_bool(data.eq[0][i * 4] as u16);
            // step_addr
        }
        unimplemented!()
    }
    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness<F: PrimeField64>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<ArithEqInput>],
    ) -> AirInstance<F> {
        // Get the fixed cols
        let _airgroup_id = ArithEqTrace::<usize>::AIRGROUP_ID;
        let _air_id = ArithEqTrace::<usize>::AIR_ID;

        let mut trace = ArithEqTrace::<F>::new();
        let num_rows = trace.num_rows();
        let num_rows_needed = inputs.len() * ARITH_EQ_ROWS_BY_OP;

        info!(
            "{}: ··· Creating ArithEq instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(ARITH_EQ_TRACE);

        let arith256 = executors::Arith256::new();
        let arith256_mod = executors::Arith256Mod::new();
        let secp256k1 = executors::Secp256k1::new();

        let mut index = 0;
        for inputs in inputs.iter() {
            for input in inputs.iter() {
                let row_offset = index * ARITH_EQ_ROWS_BY_OP;
                match input {
                    ArithEqInput::Arith256(idata) => {
                        Self::process_arith256(&arith256, idata, &mut trace, row_offset)
                    }
                    ArithEqInput::Arith256Mod(idata) => {
                        Self::process_arith256_mod(&arith256_mod, idata, &mut trace, row_offset)
                    }
                    ArithEqInput::Secp256k1Add(idata) => {
                        Self::process_secp256k1_add(&secp256k1, idata, &mut trace, row_offset)
                    }
                    ArithEqInput::Secp256k1Dbl(idata) => {
                        Self::process_secp256k1_dbl(&secp256k1, idata, &mut trace, row_offset)
                    }
                }
                index += 1;
            }
        }
        timer_stop_and_log_trace!(ARITH_EQ_TRACE);

        unimplemented!()
    }
}
