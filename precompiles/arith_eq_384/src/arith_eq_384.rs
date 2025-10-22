use fields::PrimeField64;
use rayon::prelude::*;
use std::sync::Arc;

use pil_std_lib::Std;
use precomp_arith_eq::ArithEqLtTableSM;
use proofman_common::{AirInstance, FromTrace, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
#[cfg(not(feature = "packed"))]
use zisk_pil::{ArithEq384Trace, ArithEq384TraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{ArithEq384TracePacked, ArithEq384TraceRowPacked};

#[cfg(feature = "packed")]
type ArithEq384TraceRowType<F> = ArithEq384TraceRowPacked<F>;
#[cfg(feature = "packed")]
type ArithEq384TraceType<F> = ArithEq384TracePacked<F>;

#[cfg(not(feature = "packed"))]
type ArithEq384TraceRowType<F> = ArithEq384TraceRow<F>;
#[cfg(not(feature = "packed"))]
type ArithEq384TraceType<F> = ArithEq384Trace<F>;

use crate::{
    arith_eq_384_constants::*, executors, Arith384ModInput, ArithEq384Input,
    Bls12_381ComplexAddInput, Bls12_381ComplexMulInput, Bls12_381ComplexSubInput,
    Bls12_381CurveAddInput, Bls12_381CurveDblInput,
};

/// The `ArithEq384SM` struct encapsulates the logic of the ArithEq384 State Machine.
pub struct ArithEq384SM<F: PrimeField64> {
    /// Number of available arith384s in the trace.
    pub num_available_ops: usize,

    num_non_usable_rows: usize,

    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// The table ID for the Keccakf Table State Machine
    table_id: usize,

    pub q_hsc_range_id: usize,
    pub chunk_range_id: usize,
    pub carry_range_id: usize,
}
#[derive(Debug, Default)]
struct ArithEq384StepAddr {
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

impl<F: PrimeField64> ArithEq384SM<F> {
    /// Creates a new ArithEq384 State Machine instance.
    ///
    /// # Returns
    /// A new `ArithEq384SM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Compute some useful values
        let num_available_ops = ArithEq384TraceType::<F>::NUM_ROWS / ARITH_EQ_384_ROWS_BY_OP - 1;
        let num_non_usable_rows = ArithEq384TraceType::<F>::NUM_ROWS % ARITH_EQ_384_ROWS_BY_OP;
        let q_hsc_range_id = std.get_range_id(0, ARITH_EQ_384_Q_HSC_MAX, None);
        let chunk_range_id = std.get_range_id(0, ARITH_EQ_384_CHUNK_MAX as i64, None);
        let carry_range_id = std.get_range_id(ARITH_EQ_384_CARRY_MIN, ARITH_EQ_384_CARRY_MAX, None);

        // Get the table ID
        let table_id = std.get_virtual_table_id(ArithEqLtTableSM::TABLE_ID);

        Arc::new(Self {
            std,
            num_available_ops,
            num_non_usable_rows,
            q_hsc_range_id,
            chunk_range_id,
            carry_range_id,
            table_id,
        })
    }

    fn expand_addr_step_on_trace(
        data: &ArithEq384StepAddr,
        trace: &mut [ArithEq384TraceRowType<F>],
    ) {
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
        for i in 0..(ARITH_EQ_384_ROWS_BY_OP - 8 - data.addr_ind.len()) {
            trace[i + 8 + data.addr_ind.len()].set_step_addr(0);
        }
    }

    fn process_arith384_mod(
        &self,
        input: &Arith384ModInput,
        trace: &mut [ArithEq384TraceRowType<F>],
    ) {
        let data = executors::Arith384Mod::execute(&input.a, &input.b, &input.c, &input.module);
        self.expand_data_on_trace(&data, trace, SEL_OP_ARITH384_MOD);
        Self::expand_addr_step_on_trace(
            &ArithEq384StepAddr {
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

    fn process_bls12_381_curve_add(
        &self,
        input: &Bls12_381CurveAddInput,
        trace: &mut [ArithEq384TraceRowType<F>],
    ) {
        let data = executors::Bls12_381Curve::execute_add(&input.p1, &input.p2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BLS12_381_CURVE_ADD);
        Self::expand_addr_step_on_trace(
            &ArithEq384StepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.p1_addr,
                addr_y1: input.p1_addr + 48,
                addr_x2: input.p2_addr,
                addr_y2: input.p2_addr + 48,
                addr_x3: input.p1_addr,
                addr_y3: input.p1_addr + 48,
                addr_ind: [input.p1_addr, input.p2_addr, 0, 0, 0],
            },
            trace,
        );
    }

    fn process_bls12_381_curve_dbl(
        &self,
        input: &Bls12_381CurveDblInput,
        trace: &mut [ArithEq384TraceRowType<F>],
    ) {
        let data = executors::Bls12_381Curve::execute_dbl(&input.p1);
        self.expand_data_on_trace(&data, trace, SEL_OP_BLS12_381_CURVE_DBL);
        Self::expand_addr_step_on_trace(
            &ArithEq384StepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.addr,
                addr_y1: input.addr + 48,
                addr_x2: input.addr,
                addr_y2: input.addr + 48,
                addr_x3: input.addr,
                addr_y3: input.addr + 48,
                addr_ind: [0, 0, 0, 0, 0],
            },
            trace,
        );
    }

    fn process_bls12_381_complex_add(
        &self,
        input: &Bls12_381ComplexAddInput,
        trace: &mut [ArithEq384TraceRowType<F>],
    ) {
        let data = executors::Bls12_381Complex::execute_add(&input.f1, &input.f2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BLS12_381_COMPLEX_ADD);
        Self::expand_addr_step_on_trace(
            &ArithEq384StepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.f1_addr,
                addr_y1: input.f1_addr + 48,
                addr_x2: input.f2_addr,
                addr_y2: input.f2_addr + 48,
                addr_x3: input.f1_addr,
                addr_y3: input.f1_addr + 48,
                addr_ind: [input.f1_addr, input.f2_addr, 0, 0, 0],
            },
            trace,
        );
    }

    fn process_bls12_381_complex_sub(
        &self,
        input: &Bls12_381ComplexSubInput,
        trace: &mut [ArithEq384TraceRowType<F>],
    ) {
        let data = executors::Bls12_381Complex::execute_sub(&input.f1, &input.f2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BLS12_381_COMPLEX_SUB);
        Self::expand_addr_step_on_trace(
            &ArithEq384StepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.f1_addr,
                addr_y1: input.f1_addr + 48,
                addr_x2: input.f2_addr,
                addr_y2: input.f2_addr + 48,
                addr_x3: input.f1_addr,
                addr_y3: input.f1_addr + 48,
                addr_ind: [input.f1_addr, input.f2_addr, 0, 0, 0],
            },
            trace,
        );
    }

    fn process_bls12_381_complex_mul(
        &self,
        input: &Bls12_381ComplexMulInput,
        trace: &mut [ArithEq384TraceRowType<F>],
    ) {
        let data = executors::Bls12_381Complex::execute_mul(&input.f1, &input.f2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BLS12_381_COMPLEX_MUL);
        Self::expand_addr_step_on_trace(
            &ArithEq384StepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.f1_addr,
                addr_y1: input.f1_addr + 48,
                addr_x2: input.f2_addr,
                addr_y2: input.f2_addr + 48,
                addr_x3: input.f1_addr,
                addr_y3: input.f1_addr + 48,
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
        data: &executors::ArithEq384Data,
        trace: &mut [ArithEq384TraceRowType<F>],
        sel_op: usize,
    ) {
        let mut x1_x2_different = false;
        let mut prev_x3_lt = false;
        let mut prev_y3_lt = false;

        #[allow(clippy::needless_range_loop)]
        for i in 0..ARITH_EQ_384_ROWS_BY_OP {
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
            let q_range_id = if i == ARITH_EQ_384_ROWS_BY_OP - 1 {
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
            for j in 0..ARITH_EQ_384_OP_NUM {
                let selected = j == sel_op;
                trace[i].set_sel_op(j, selected);
                if i == 0 {
                    trace[i].set_sel_op_clk0(j, selected);
                } else {
                    trace[i].set_sel_op_clk0(j, false);
                }
            }
            match sel_op {
                SEL_OP_ARITH384_MOD => {
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
                SEL_OP_BLS12_381_CURVE_ADD
                | SEL_OP_BLS12_381_CURVE_DBL
                | SEL_OP_BLS12_381_COMPLEX_ADD
                | SEL_OP_BLS12_381_COMPLEX_SUB
                | SEL_OP_BLS12_381_COMPLEX_MUL => {
                    let x3_lt = data.x3[i] < BLS12_381_PRIME_CHUNKS[i]
                        || (data.x3[i] == BLS12_381_PRIME_CHUNKS[i] && prev_x3_lt);
                    trace[i].set_x3_lt(x3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_x3_lt,
                        x3_lt,
                        data.x3[i] - BLS12_381_PRIME_CHUNKS[i],
                    );
                    self.std.inc_virtual_row(self.table_id, row as u64, 1);
                    prev_x3_lt = x3_lt;

                    let y3_lt = data.y3[i] < BLS12_381_PRIME_CHUNKS[i]
                        || (data.y3[i] == BLS12_381_PRIME_CHUNKS[i] && prev_y3_lt);
                    trace[i].set_y3_lt(y3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_y3_lt,
                        y3_lt,
                        data.y3[i] - BLS12_381_PRIME_CHUNKS[i],
                    );
                    self.std.inc_virtual_row(self.table_id, row as u64, 1);
                    prev_y3_lt = y3_lt;
                }
                _ => {
                    trace[i].set_x3_lt(false);
                    trace[i].set_y3_lt(false);
                }
            }
            if sel_op == SEL_OP_BLS12_381_CURVE_ADD {
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
                    trace[i].set_x_delta_chunk_inv(0);
                    trace[i].set_x_are_different(false);
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
        inputs: &[Vec<ArithEq384Input>],
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut trace = ArithEq384TraceType::new_from_vec(trace_buffer);
        let num_rows = trace.num_rows();
        let num_available_ops = self.num_available_ops;

        let total_inputs: usize = inputs.iter().map(|x| x.len()).sum();
        let num_rows_filled = total_inputs * ARITH_EQ_384_ROWS_BY_OP;
        let num_rows_needed = if total_inputs < num_available_ops {
            total_inputs * ARITH_EQ_384_ROWS_BY_OP
        } else if total_inputs == num_available_ops {
            num_rows
        } else {
            panic!(
                "Exceeded available ArithEq384 inputs: requested {}, but only {} are available.",
                total_inputs, num_available_ops
            );
        };

        tracing::info!(
            "··· Creating ArithEq384 instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(ARITH_EQ_384_TRACE);

        let mut trace_rows = &mut trace.buffer[..];
        let mut par_traces = Vec::new();
        let mut inputs_indexes = Vec::new();
        for (i, inputs) in inputs.iter().enumerate() {
            for (j, _) in inputs.iter().enumerate() {
                let (head, tail) = trace_rows.split_at_mut(ARITH_EQ_384_ROWS_BY_OP);
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
                ArithEq384Input::Arith384Mod(idata) => self.process_arith384_mod(idata, trace),
                ArithEq384Input::Bls12_381CurveAdd(idata) => {
                    self.process_bls12_381_curve_add(idata, trace)
                }
                ArithEq384Input::Bls12_381CurveDbl(idata) => {
                    self.process_bls12_381_curve_dbl(idata, trace)
                }
                ArithEq384Input::Bls12_381ComplexAdd(idata) => {
                    self.process_bls12_381_complex_add(idata, trace);
                }
                ArithEq384Input::Bls12_381ComplexSub(idata) => {
                    self.process_bls12_381_complex_sub(idata, trace);
                }
                ArithEq384Input::Bls12_381ComplexMul(idata) => {
                    self.process_bls12_381_complex_mul(idata, trace);
                }
            }
        });

        let num_non_usable_rows = self.num_non_usable_rows;
        let padding_ops = (num_available_ops - index) as u64;
        let q_hsc_range_mult = 3 * padding_ops; // 3 q_cols by each ARITH_EQ_384_ROWS_BY_OP
        let chunk_range_mult = (7 * ARITH_EQ_384_ROWS_BY_OP as u64
            + 3 * (ARITH_EQ_384_ROWS_BY_OP - 1) as u64)
            * padding_ops
            + 7 * (ARITH_EQ_384_ROWS_BY_OP + num_non_usable_rows) as u64
            + 3 * (ARITH_EQ_384_ROWS_BY_OP + num_non_usable_rows) as u64; // 7 chunk_cols + 3 q_cols
        let carry_range_mult = (6 * ARITH_EQ_384_ROWS_BY_OP as u64) * padding_ops
            + 6 * (ARITH_EQ_384_ROWS_BY_OP + num_non_usable_rows) as u64; // 6 carry_cols
        self.std.range_check(self.q_hsc_range_id, 0, q_hsc_range_mult);
        self.std.range_check(self.chunk_range_id, 0, chunk_range_mult);
        self.std.range_check(self.carry_range_id, 0, carry_range_mult);

        let padding_row = ArithEq384TraceRowType::default();

        trace.buffer[num_rows_filled..num_rows].par_iter_mut().for_each(|slot| *slot = padding_row);

        timer_stop_and_log_trace!(ARITH_EQ_384_TRACE);

        AirInstance::new_from_trace(FromTrace::new(&mut trace))
    }
}
