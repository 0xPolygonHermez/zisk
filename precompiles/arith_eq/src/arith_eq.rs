use fields::PrimeField64;
use std::sync::Arc;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
#[cfg(not(feature = "packed"))]
use zisk_pil::{ArithEqTrace, ArithEqTraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{ArithEqTracePacked, ArithEqTraceRowPacked};

#[cfg(feature = "packed")]
type ArithEqTraceRowType<F> = ArithEqTraceRowPacked<F>;
#[cfg(feature = "packed")]
type ArithEqTraceType<F> = ArithEqTracePacked<F>;

#[cfg(not(feature = "packed"))]
type ArithEqTraceRowType<F> = ArithEqTraceRow<F>;
#[cfg(not(feature = "packed"))]
type ArithEqTraceType<F> = ArithEqTrace<F>;

use crate::{
    arith_eq_constants::*, executors, Arith256Input, Arith256ModInput, ArithEqInput,
    ArithEqLtTableSM, Bn254ComplexAddInput, Bn254ComplexMulInput, Bn254ComplexSubInput,
    Bn254CurveAddInput, Bn254CurveDblInput, Secp256k1AddInput, Secp256k1DblInput,
    SECP256K1_PRIME_CHUNKS, SEL_OP_ARITH256, SEL_OP_ARITH256_MOD, SEL_OP_SECP256K1_ADD,
    SEL_OP_SECP256K1_DBL,
};
use rayon::prelude::*;

/// The `ArithEqSM` struct encapsulates the logic of the ArithEq State Machine.
pub struct ArithEqSM<F: PrimeField64> {
    /// Number of available arith256s in the trace.
    pub num_available_ops: usize,

    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// The table ID for the Keccakf Table State Machine
    table_id: usize,

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
    /// Creates a new ArithEq State Machine instance.
    ///
    /// # Returns
    /// A new `ArithEqSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Compute some useful values
        let num_available_ops = ArithEqTraceType::<F>::NUM_ROWS / ARITH_EQ_ROWS_BY_OP;
        let p2_22 = 1 << 22;
        let q_hsc_range_id = std.get_range_id(0, p2_22 - 1, None);
        let chunk_range_id = std.get_range_id(0, 0xFFFF, None);
        let carry_range_id = std.get_range_id(-(p2_22 - 1), p2_22, None);

        // Get the table ID
        let table_id = std.get_virtual_table_id(ArithEqLtTableSM::TABLE_ID);

        Arc::new(Self {
            std,
            num_available_ops,
            q_hsc_range_id,
            chunk_range_id,
            carry_range_id,
            table_id,
        })
    }
    fn expand_addr_step_on_trace(data: &ArithEqStepAddr, trace: &mut [ArithEqTraceRowType<F>]) {
        trace[0].set_step_addr(data.main_step);
        trace[1].set_step_addr(data.addr_op as u64);
        trace[2].set_step_addr(data.addr_x1 as u64);
        trace[3].set_step_addr(data.addr_y1 as u64);
        trace[4].set_step_addr(data.addr_x2 as u64);
        trace[5].set_step_addr(data.addr_y2 as u64);
        trace[6].set_step_addr(data.addr_x3 as u64);
        trace[7].set_step_addr(data.addr_y3 as u64);
        for (i, addr_ind) in data.addr_ind.iter().enumerate() {
            trace[i + 8].set_step_addr(*addr_ind as u64);
        }
        for i in 0..(ARITH_EQ_ROWS_BY_OP - 8 - data.addr_ind.len()) {
            trace[i + 8 + data.addr_ind.len()].set_step_addr(0);
        }
    }

    fn process_arith256(&self, input: &Arith256Input, trace: &mut [ArithEqTraceRowType<F>]) {
        let data = executors::Arith256::execute(&input.a, &input.b, &input.c);
        self.expand_data_on_trace(&data, trace, SEL_OP_ARITH256);
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
        );
    }

    fn process_arith256_mod(&self, input: &Arith256ModInput, trace: &mut [ArithEqTraceRowType<F>]) {
        let data = executors::Arith256Mod::execute(&input.a, &input.b, &input.c, &input.module);
        self.expand_data_on_trace(&data, trace, SEL_OP_ARITH256_MOD);
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
        );
    }
    fn process_secp256k1_add(
        &self,
        input: &Secp256k1AddInput,
        trace: &mut [ArithEqTraceRowType<F>],
    ) {
        let data = executors::Secp256k1::execute_add(&input.p1, &input.p2);
        self.expand_data_on_trace(&data, trace, SEL_OP_SECP256K1_ADD);
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
        );
    }
    fn process_secp256k1_dbl(
        &self,
        input: &Secp256k1DblInput,
        trace: &mut [ArithEqTraceRowType<F>],
    ) {
        let data = executors::Secp256k1::execute_dbl(&input.p1);
        self.expand_data_on_trace(&data, trace, SEL_OP_SECP256K1_DBL);
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
        );
    }

    fn process_bn254_curve_add(
        &self,
        input: &Bn254CurveAddInput,
        trace: &mut [ArithEqTraceRowType<F>],
    ) {
        let data = executors::Bn254Curve::execute_add(&input.p1, &input.p2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BN254_CURVE_ADD);
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
        );
    }

    fn process_bn254_curve_dbl(
        &self,
        input: &Bn254CurveDblInput,
        trace: &mut [ArithEqTraceRowType<F>],
    ) {
        let data = executors::Bn254Curve::execute_dbl(&input.p1);
        self.expand_data_on_trace(&data, trace, SEL_OP_BN254_CURVE_DBL);
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
        );
    }

    fn process_bn254_complex_add(
        &self,
        input: &Bn254ComplexAddInput,
        trace: &mut [ArithEqTraceRowType<F>],
    ) {
        let data = executors::Bn254Complex::execute_add(&input.f1, &input.f2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BN254_COMPLEX_ADD);
        Self::expand_addr_step_on_trace(
            &ArithEqStepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.f1_addr,
                addr_y1: input.f1_addr + 32,
                addr_x2: input.f2_addr,
                addr_y2: input.f2_addr + 32,
                addr_x3: input.f1_addr,
                addr_y3: input.f1_addr + 32,
                addr_ind: [input.f1_addr, input.f2_addr, 0, 0, 0],
            },
            trace,
        );
    }

    fn process_bn254_complex_sub(
        &self,
        input: &Bn254ComplexSubInput,
        trace: &mut [ArithEqTraceRowType<F>],
    ) {
        let data = executors::Bn254Complex::execute_sub(&input.f1, &input.f2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BN254_COMPLEX_SUB);
        Self::expand_addr_step_on_trace(
            &ArithEqStepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.f1_addr,
                addr_y1: input.f1_addr + 32,
                addr_x2: input.f2_addr,
                addr_y2: input.f2_addr + 32,
                addr_x3: input.f1_addr,
                addr_y3: input.f1_addr + 32,
                addr_ind: [input.f1_addr, input.f2_addr, 0, 0, 0],
            },
            trace,
        );
    }

    fn process_bn254_complex_mul(
        &self,
        input: &Bn254ComplexMulInput,
        trace: &mut [ArithEqTraceRowType<F>],
    ) {
        let data = executors::Bn254Complex::execute_mul(&input.f1, &input.f2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BN254_COMPLEX_MUL);
        Self::expand_addr_step_on_trace(
            &ArithEqStepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.f1_addr,
                addr_y1: input.f1_addr + 32,
                addr_x2: input.f2_addr,
                addr_y2: input.f2_addr + 32,
                addr_x3: input.f1_addr,
                addr_y3: input.f1_addr + 32,
                addr_ind: [input.f1_addr, input.f2_addr, 0, 0, 0],
            },
            trace,
        );
    }

    #[inline(always)]
    fn to_ranged_field(&self, value: i64, range_id: usize) -> u64 {
        self.std.range_check(range_id, value, 1);
        if value >= 0 {
            value as u64
        } else {
            (F::ORDER_U64 as i64 + value) as u64
        }
    }

    fn expand_data_on_trace(
        &self,
        data: &executors::ArithEqData,
        trace: &mut [ArithEqTraceRowType<F>],
        sel_op: usize,
    ) {
        let mut x1_x2_different = false;
        let mut prev_x3_lt = false;
        let mut prev_y3_lt = false;

        #[allow(clippy::needless_range_loop)]
        for i in 0..ARITH_EQ_ROWS_BY_OP {
            for j in 0..3 {
                // first position without carry
                let carry_0 = if i == 0 { 0 } else { data.cout[i * 2 - 1][j] };
                trace[i].set_carry(j, 0, self.to_ranged_field(carry_0, self.carry_range_id));
                trace[i].set_carry(
                    j,
                    1,
                    self.to_ranged_field(data.cout[i * 2][j], self.carry_range_id),
                );
            }
            let q_range_id = if i == ARITH_EQ_ROWS_BY_OP - 1 {
                self.q_hsc_range_id
            } else {
                self.chunk_range_id
            };
            trace[i].set_x1(self.to_ranged_field(data.x1[i], self.chunk_range_id) as u16);
            trace[i].set_y1(self.to_ranged_field(data.y1[i], self.chunk_range_id) as u16);
            trace[i].set_x2(self.to_ranged_field(data.x2[i], self.chunk_range_id) as u16);
            trace[i].set_y2(self.to_ranged_field(data.y2[i], self.chunk_range_id) as u16);
            trace[i].set_x3(self.to_ranged_field(data.x3[i], self.chunk_range_id) as u16);
            trace[i].set_y3(self.to_ranged_field(data.y3[i], self.chunk_range_id) as u16);
            trace[i].set_q0(self.to_ranged_field(data.q0[i], q_range_id) as u32);
            trace[i].set_q1(self.to_ranged_field(data.q1[i], q_range_id) as u32);
            trace[i].set_q2(self.to_ranged_field(data.q2[i], q_range_id) as u32);
            trace[i].set_s(self.to_ranged_field(data.s[i], self.chunk_range_id) as u32);

            // TODO Range check
            for j in 0..ARITH_EQ_OP_NUM {
                let selected = j == sel_op;
                trace[i].set_sel_op(j, selected);
                if i == 0 {
                    trace[i].set_sel_op_clk0(j, selected);
                } else {
                    trace[i].set_sel_op_clk0(j, false);
                }
            }
            match sel_op {
                SEL_OP_ARITH256_MOD => {
                    let x3_lt = data.x3[i] < data.y2[i] || (data.x3[i] == data.y2[i] && prev_x3_lt);
                    trace[i].set_x3_lt(x3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_x3_lt,
                        x3_lt,
                        data.x3[i] - data.y2[i],
                    );
                    self.std.inc_virtual_row(self.table_id, row as u64, 1);
                    prev_x3_lt = x3_lt;

                    trace[i].set_y3_lt(false);
                }
                SEL_OP_SECP256K1_ADD | SEL_OP_SECP256K1_DBL => {
                    let x3_lt = data.x3[i] < SECP256K1_PRIME_CHUNKS[i]
                        || (data.x3[i] == SECP256K1_PRIME_CHUNKS[i] && prev_x3_lt);
                    trace[i].set_x3_lt(x3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_x3_lt,
                        x3_lt,
                        data.x3[i] - SECP256K1_PRIME_CHUNKS[i],
                    );
                    self.std.inc_virtual_row(self.table_id, row as u64, 1);
                    prev_x3_lt = x3_lt;

                    let y3_lt = data.y3[i] < SECP256K1_PRIME_CHUNKS[i]
                        || (data.y3[i] == SECP256K1_PRIME_CHUNKS[i] && prev_y3_lt);
                    trace[i].set_y3_lt(y3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_y3_lt,
                        y3_lt,
                        data.y3[i] - SECP256K1_PRIME_CHUNKS[i],
                    );
                    self.std.inc_virtual_row(self.table_id, row as u64, 1);
                    prev_y3_lt = y3_lt;
                }
                SEL_OP_BN254_CURVE_ADD
                | SEL_OP_BN254_CURVE_DBL
                | SEL_OP_BN254_COMPLEX_ADD
                | SEL_OP_BN254_COMPLEX_SUB
                | SEL_OP_BN254_COMPLEX_MUL => {
                    let x3_lt = data.x3[i] < BN254_PRIME_CHUNKS[i]
                        || (data.x3[i] == BN254_PRIME_CHUNKS[i] && prev_x3_lt);
                    trace[i].set_x3_lt(x3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_x3_lt,
                        x3_lt,
                        data.x3[i] - BN254_PRIME_CHUNKS[i],
                    );
                    self.std.inc_virtual_row(self.table_id, row as u64, 1);
                    prev_x3_lt = x3_lt;

                    let y3_lt = data.y3[i] < BN254_PRIME_CHUNKS[i]
                        || (data.y3[i] == BN254_PRIME_CHUNKS[i] && prev_y3_lt);
                    trace[i].set_y3_lt(y3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_y3_lt,
                        y3_lt,
                        data.y3[i] - BN254_PRIME_CHUNKS[i],
                    );
                    self.std.inc_virtual_row(self.table_id, row as u64, 1);
                    prev_y3_lt = y3_lt;
                }
                _ => {
                    trace[i].set_x3_lt(false);
                    trace[i].set_y3_lt(false);
                }
            }
            if (sel_op == SEL_OP_SECP256K1_ADD) || (sel_op == SEL_OP_BN254_CURVE_ADD) {
                if x1_x2_different {
                    trace[i].set_x_are_different(true);
                    trace[i].set_x_delta_chunk_inv(0);
                } else if data.x1[i] != data.x2[i] {
                    x1_x2_different = true;
                    trace[i].set_x_are_different(true);
                    trace[i].set_x_delta_chunk_inv(
                        F::inverse(&F::from_i64(data.x2[i] - data.x1[i])).as_canonical_u64(),
                    );
                } else {
                    trace[i].set_x_are_different(false);
                    trace[i].set_x_delta_chunk_inv(0);
                }
            } else {
                trace[i].set_x_are_different(false);
                trace[i].set_x_delta_chunk_inv(0);
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
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut trace = ArithEqTraceType::new_from_vec(trace_buffer);
        let num_rows = trace.num_rows();
        let total_inputs: usize = inputs.iter().map(|x| x.len()).sum();
        let num_rows_needed = total_inputs * ARITH_EQ_ROWS_BY_OP;

        tracing::info!(
            "··· Creating ArithEq instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(ARITH_EQ_TRACE);

        let mut trace_rows = &mut trace.buffer[..];
        let mut par_traces = Vec::new();
        let mut inputs_indexes = Vec::new();
        for (i, inputs) in inputs.iter().enumerate() {
            for (j, _) in inputs.iter().enumerate() {
                let (head, tail) = trace_rows.split_at_mut(ARITH_EQ_ROWS_BY_OP);
                par_traces.push(head);
                inputs_indexes.push((i, j));
                trace_rows = tail;
            }
        }
        let index = par_traces.len();

        par_traces.into_par_iter().enumerate().for_each(|(index, trace)| {
            let input_index = inputs_indexes[index];
            let input = &inputs[input_index.0][input_index.1];
            match input {
                ArithEqInput::Arith256(idata) => self.process_arith256(idata, trace),
                ArithEqInput::Arith256Mod(idata) => self.process_arith256_mod(idata, trace),
                ArithEqInput::Secp256k1Add(idata) => self.process_secp256k1_add(idata, trace),
                ArithEqInput::Secp256k1Dbl(idata) => self.process_secp256k1_dbl(idata, trace),
                ArithEqInput::Bn254CurveAdd(idata) => self.process_bn254_curve_add(idata, trace),
                ArithEqInput::Bn254CurveDbl(idata) => self.process_bn254_curve_dbl(idata, trace),
                ArithEqInput::Bn254ComplexAdd(idata) => {
                    self.process_bn254_complex_add(idata, trace);
                }
                ArithEqInput::Bn254ComplexSub(idata) => {
                    self.process_bn254_complex_sub(idata, trace);
                }
                ArithEqInput::Bn254ComplexMul(idata) => {
                    self.process_bn254_complex_mul(idata, trace);
                }
            }
        });

        let padding_ops = (self.num_available_ops - index) as u64;
        self.std.range_check(self.q_hsc_range_id, 0, 3 * padding_ops);
        self.std.range_check(self.chunk_range_id, 0, 157 * padding_ops);
        self.std.range_check(self.carry_range_id, 0, 96 * padding_ops);

        let padding_row = ArithEqTraceRowType::default();

        trace.buffer[num_rows_needed..num_rows].par_iter_mut().for_each(|slot| *slot = padding_row);

        timer_stop_and_log_trace!(ARITH_EQ_TRACE);

        AirInstance::new_from_trace(FromTrace::new(&mut trace))
    }
}
