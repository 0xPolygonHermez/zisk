use std::sync::Arc;

use log::info;
use p3_field::PrimeField64;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::ArithEqTrace;

use crate::{
    arith_eq_constants::*, executors, Arith256Input, Arith256ModInput, ArithEqInput,
    ArithEqLtTableSM, Secp256k1AddInput, Secp256k1DblInput,
};

const SEL_OP_ARITH256: usize = 0;
const SEL_OP_ARITH256_MOD: usize = 1;
const SEL_OP_SECP256K1_ADD: usize = 2;
const SEL_OP_SECP256K1_DBL: usize = 3;

const SECP256K1_PRIME_CHUNKS: [i64; 16] = [
    0xFC2F, 0xFFFF, 0xFFFE, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
    0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
];
/// The `ArithEqSM` struct encapsulates the logic of the ArithEq State Machine.
pub struct ArithEqSM<F: PrimeField64> {
    /// Number of available arith256s in the trace.
    pub num_available_ops: usize,

    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    pub arith_eq_lt_table_sm: Arc<ArithEqLtTableSM>,

    pub q_hsc_range_id: usize,
    pub chunk_range_id: usize,
    pub carry_range_id: usize,
}
#[derive(Debug, Default)]
struct ArithEqStepAddr {
    main_step: u64,
    addr_op: u32,
    addr_x1: u32,
    addr_y1: u32,
    addr_x2: u32,
    addr_y2: u32,
    addr_x3: u32,
    addr_y3: u32,
    addr_ind: [u32; 5],
}

impl<F: PrimeField64> ArithEqSM<F> {
    const MY_NAME: &'static str = "ArithEq  ";

    /// Creates a new ArithEq State Machine instance.
    ///
    /// # Returns
    /// A new `ArithEqSM` instance.
    pub fn new(std: Arc<Std<F>>, arith_eq_lt_table_sm: Arc<ArithEqLtTableSM>) -> Arc<Self> {
        // Compute some useful values
        let num_available_ops = ArithEqTrace::<usize>::NUM_ROWS / ARITH_EQ_ROWS_BY_OP;
        let p2_22 = 1 << 22;
        let q_hsc_range_id = std.get_range(0, p2_22 - 1, None);
        let chunk_range_id = std.get_range(0, 0xFFFF, None);
        let carry_range_id = std.get_range(-(p2_22 - 1), p2_22, None);

        Arc::new(Self {
            std,
            num_available_ops,
            q_hsc_range_id,
            chunk_range_id,
            carry_range_id,
            arith_eq_lt_table_sm,
        })
    }
    fn expand_addr_step_on_trace(
        data: &ArithEqStepAddr,
        trace: &mut ArithEqTrace<F>,
        row_offset: usize,
    ) {
        trace[row_offset].step_addr = F::from_u64(data.main_step);
        trace[row_offset + 1].step_addr = F::from_u32(data.addr_op);
        trace[row_offset + 2].step_addr = F::from_u32(data.addr_x1);
        trace[row_offset + 3].step_addr = F::from_u32(data.addr_y1);
        trace[row_offset + 4].step_addr = F::from_u32(data.addr_x2);
        trace[row_offset + 5].step_addr = F::from_u32(data.addr_y2);
        trace[row_offset + 6].step_addr = F::from_u32(data.addr_x3);
        trace[row_offset + 7].step_addr = F::from_u32(data.addr_y3);
        for (i, addr_ind) in data.addr_ind.iter().enumerate() {
            trace[row_offset + i + 8].step_addr = F::from_u32(*addr_ind);
        }
    }

    fn process_arith256(
        &self,
        input: &Arith256Input,
        trace: &mut ArithEqTrace<F>,
        row_offset: usize,
    ) {
        let data = executors::Arith256::execute(&input.a, &input.b, &input.c);
        self.expand_data_on_trace(&data, row_offset, trace, 0);
        Self::expand_addr_step_on_trace(
            &ArithEqStepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.a_addr,
                addr_y1: input.b_addr,
                addr_x2: input.c_addr,
                addr_y2: 0,
                addr_x3: input.dl_addr,
                addr_y3: input.dh_addr,
                addr_ind: [input.a_addr, input.b_addr, input.c_addr, input.dl_addr, input.dh_addr],
            },
            trace,
            row_offset,
        );
    }

    fn process_arith256_mod(
        &self,
        input: &Arith256ModInput,
        trace: &mut ArithEqTrace<F>,
        row_offset: usize,
    ) {
        let data = executors::Arith256Mod::execute(&input.a, &input.b, &input.c, &input.module);
        self.expand_data_on_trace(&data, row_offset, trace, 1);
        Self::expand_addr_step_on_trace(
            &ArithEqStepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.a_addr,
                addr_y1: input.b_addr,
                addr_x2: input.c_addr,
                addr_y2: input.module_addr,
                addr_x3: input.d_addr,
                addr_y3: 0,
                addr_ind: [
                    input.a_addr,
                    input.b_addr,
                    input.c_addr,
                    input.module_addr,
                    input.d_addr,
                ],
            },
            trace,
            row_offset,
        );
    }
    fn process_secp256k1_add(
        &self,
        input: &Secp256k1AddInput,
        trace: &mut ArithEqTrace<F>,
        row_offset: usize,
    ) {
        let data = executors::Secp256k1::execute_add(&input.p1, &input.p2);
        self.expand_data_on_trace(&data, row_offset, trace, 2);
        Self::expand_addr_step_on_trace(
            &ArithEqStepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.p1_addr,
                addr_y1: input.p1_addr + 32,
                addr_x2: input.p2_addr,
                addr_y2: input.p2_addr + 32,
                addr_x3: input.p1_addr,
                addr_y3: input.p1_addr + 32,
                addr_ind: [input.p1_addr, input.p2_addr, 0, 0, 0],
            },
            trace,
            row_offset,
        );
    }
    fn process_secp256k1_dbl(
        &self,
        input: &Secp256k1DblInput,
        trace: &mut ArithEqTrace<F>,
        row_offset: usize,
    ) {
        let data = executors::Secp256k1::execute_dbl(&input.p1);
        self.expand_data_on_trace(&data, row_offset, trace, 3);
        Self::expand_addr_step_on_trace(
            &ArithEqStepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.addr,
                addr_y1: input.addr + 32,
                addr_x2: input.addr,
                addr_y2: input.addr + 32,
                addr_x3: input.addr,
                addr_y3: input.addr + 32,
                addr_ind: [0, 0, 0, 0, 0],
            },
            trace,
            row_offset,
        );
    }

    #[inline(always)]
    fn to_ranged_field(&self, value: i64, range_id: usize) -> F {
        self.std.range_check(value, 1, range_id);
        F::from_i64(value)
    }
    fn expand_data_on_trace(
        &self,
        data: &executors::ArithEqData,
        row_offset: usize,
        trace: &mut ArithEqTrace<F>,
        sel_op: usize,
    ) {
        let mut x1_x2_different = false;
        let mut prev_x3_lt = false;
        let mut prev_y3_lt = false;
        for i in 0..ARITH_EQ_ROWS_BY_OP {
            let irow = row_offset + i;
            for j in 0..3 {
                // first position without carry
                let carry_0 = if i == 0 { 0 } else { data.cout[i * 2 - 1][j] };
                trace[irow].carry[j][0] = self.to_ranged_field(carry_0, self.carry_range_id);
                trace[irow].carry[j][1] =
                    self.to_ranged_field(data.cout[i * 2][j], self.carry_range_id);
            }
            let q_range_id = if i == ARITH_EQ_ROWS_BY_OP - 1 {
                self.q_hsc_range_id
            } else {
                self.chunk_range_id
            };
            trace[irow].x1 = self.to_ranged_field(data.x1[i], self.chunk_range_id);
            trace[irow].y1 = self.to_ranged_field(data.y1[i], self.chunk_range_id);
            trace[irow].x2 = self.to_ranged_field(data.x2[i], self.chunk_range_id);
            trace[irow].y2 = self.to_ranged_field(data.y2[i], self.chunk_range_id);
            trace[irow].x3 = self.to_ranged_field(data.x3[i], self.chunk_range_id);
            trace[irow].y3 = self.to_ranged_field(data.y3[i], self.chunk_range_id);
            trace[irow].q0 = self.to_ranged_field(data.q0[i], q_range_id);
            trace[irow].q1 = self.to_ranged_field(data.q1[i], q_range_id);
            trace[irow].q2 = self.to_ranged_field(data.q2[i], q_range_id);
            trace[irow].s = self.to_ranged_field(data.s[i], self.chunk_range_id);

            // TODO Range check
            for j in 0..4 {
                let selected = j == sel_op;
                trace[irow].sel_op[j] = F::from_bool(selected);
                if i == 0 {
                    trace[irow].sel_op_clk0[j] = F::from_bool(selected);
                } else {
                    trace[irow].sel_op_clk0[j] = F::ZERO;
                }
            }
            match sel_op {
                SEL_OP_ARITH256_MOD => {
                    let x3_lt = data.x3[i] < data.y2[i];
                    trace[irow].x3_lt = F::from_bool(x3_lt);
                    self.arith_eq_lt_table_sm.update_input(
                        prev_x3_lt,
                        x3_lt,
                        data.x3[i] - data.y2[i],
                    );
                    prev_x3_lt = x3_lt;

                    trace[irow].y3_lt = F::ZERO;
                }
                SEL_OP_SECP256K1_ADD | SEL_OP_SECP256K1_DBL => {
                    let x3_lt = data.x3[i] < SECP256K1_PRIME_CHUNKS[i];
                    trace[irow].x3_lt = F::from_bool(x3_lt);
                    self.arith_eq_lt_table_sm.update_input(
                        prev_x3_lt,
                        x3_lt,
                        data.x3[i] - SECP256K1_PRIME_CHUNKS[i],
                    );
                    prev_x3_lt = x3_lt;

                    let y3_lt = data.y3[i] < SECP256K1_PRIME_CHUNKS[i];
                    trace[irow].y3_lt = F::from_bool(y3_lt);
                    self.arith_eq_lt_table_sm.update_input(
                        prev_y3_lt,
                        y3_lt,
                        data.y3[i] - SECP256K1_PRIME_CHUNKS[i],
                    );
                    prev_y3_lt = y3_lt;
                }
                _ => {
                    trace[irow].x3_lt = F::ZERO;
                    trace[irow].y3_lt = F::ZERO;
                }
            }
            if sel_op == SEL_OP_SECP256K1_ADD {
                if x1_x2_different {
                    trace[irow].x_are_different = F::ONE;
                    trace[irow].x_delta_chunk_inv = F::ZERO;
                } else if data.x1[i] != data.x2[i] {
                    x1_x2_different = true;
                    trace[irow].x_are_different = F::ONE;
                    trace[irow].x_delta_chunk_inv =
                        F::inverse(&F::from_i64(data.x2[i] - data.x1[i]));
                } else {
                    trace[irow].x_delta_chunk_inv = F::ZERO;
                    trace[irow].x_are_different = F::ZERO;
                }
            } else {
                trace[irow].x_are_different = F::ZERO;
                trace[irow].x_delta_chunk_inv = F::ZERO;
            }
        }
    }
    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness(
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

        let mut index = 0;
        for inputs in inputs.iter() {
            for input in inputs.iter() {
                let row_offset = index * ARITH_EQ_ROWS_BY_OP;
                match input {
                    ArithEqInput::Arith256(idata) => {
                        self.process_arith256(idata, &mut trace, row_offset)
                    }
                    ArithEqInput::Arith256Mod(idata) => {
                        self.process_arith256_mod(idata, &mut trace, row_offset)
                    }
                    ArithEqInput::Secp256k1Add(idata) => {
                        self.process_secp256k1_add(idata, &mut trace, row_offset)
                    }
                    ArithEqInput::Secp256k1Dbl(idata) => {
                        self.process_secp256k1_dbl(idata, &mut trace, row_offset)
                    }
                }
                index += 1;
            }
        }
        let padding_ops = (self.num_available_ops - index) as u64;
        self.std.range_check(0, 3 * padding_ops, self.q_hsc_range_id);
        self.std.range_check(0, 157 * padding_ops, self.chunk_range_id);
        self.std.range_check(0, 96 * padding_ops, self.carry_range_id);

        timer_stop_and_log_trace!(ARITH_EQ_TRACE);

        AirInstance::new_from_trace(FromTrace::new(&mut trace))
    }
}
