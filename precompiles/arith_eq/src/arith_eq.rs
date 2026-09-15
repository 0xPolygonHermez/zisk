use proofman_fields::PrimeField64;
use std::sync::Arc;

use pil2_std_lib::Std;
use proofman_common::{AirInstance, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

use crate::{
    arith_eq_constants::*, executors, Arith256Input, Arith256ModInput, ArithEqInput,
    ArithEqLtTableSM, ArithEqOp, ArithEqRow, Bn254ComplexAddInput, Bn254ComplexMulInput,
    Bn254ComplexSubInput, Bn254CurveAddInput, Bn254CurveDblInput, Secp256k1AddInput,
    Secp256k1DblInput, Secp256r1AddInput, Secp256r1DblInput, BN254_PRIME_CHUNKS,
    SECP256K1_PRIME_CHUNKS, SECP256R1_PRIME_CHUNKS, SEL_OP_ARITH256, SEL_OP_ARITH256_MOD,
    SEL_OP_SECP256K1_ADD, SEL_OP_SECP256K1_DBL, SEL_OP_SECP256R1_ADD, SEL_OP_SECP256R1_DBL,
};
// `phase_ms` / `phase_max_ms` are only read inside a `phase_log!`, which vanishes without the
// `witness_timers` feature -- and takes the only use of those imports with it.
use rayon::prelude::*;
#[allow(unused_imports)]
use zisk_common::{
    phase_end, phase_log, phase_max_ms, phase_max_record, phase_max_start, phase_ms, phase_start,
};
// `CACHE_BYTES` is only reported by the `witness_timers` line.
#[allow(unused_imports)]
use zisk_precomp_common::{MultiplicityCache, CACHE_BYTES};

/// The `ArithEqSM` struct encapsulates the logic of the ArithEq State Machine.
///
/// Nothing here depends on the height of the air: one state machine serves every config at every
/// height, and the capacity is taken from the trace each call builds.
pub struct ArithEqSM<F: PrimeField64> {
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
        let p2_22 = 1 << 22;
        let q_hsc_range_id = std.get_range_id(0, p2_22 - 1, None).expect("Failed to get range ID");
        let chunk_range_id = std.get_range_id(0, 0xFFFF, None).expect("Failed to get range ID");
        let carry_range_id =
            std.get_range_id(-(p2_22 - 1), p2_22, None).expect("Failed to get range ID");

        // Get the table ID
        let table_id =
            std.get_virtual_table_id(ArithEqLtTableSM::TABLE_ID).expect("Failed to get table ID");

        Arc::new(Self { std, q_hsc_range_id, chunk_range_id, carry_range_id, table_id })
    }
    fn get_lt_flags(input: &ArithEqInput) -> u8 {
        const X3_LT_FLAG: u8 = 1;
        const Y3_LT_FLAG: u8 = 2;
        const NO_FLAGS: u8 = 0;
        match input {
            ArithEqInput::Arith256(_) => NO_FLAGS,
            ArithEqInput::Arith256Mod(_) => X3_LT_FLAG,
            ArithEqInput::Secp256k1Add(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEqInput::Secp256k1Dbl(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEqInput::Bn254CurveAdd(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEqInput::Bn254CurveDbl(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEqInput::Bn254ComplexAdd(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEqInput::Bn254ComplexSub(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEqInput::Bn254ComplexMul(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEqInput::Secp256r1Add(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEqInput::Secp256r1Dbl(_) => X3_LT_FLAG | Y3_LT_FLAG,
        }
    }
    fn expand_addr_step_on_trace<R: ArithEqRow<F>>(data: &ArithEqStepAddr, trace: &mut [R]) {
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

    fn process_arith256<R: ArithEqRow<F>>(
        &self,
        input: &Arith256Input,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Arith256::execute(&input.a, &input.b, &input.c);
        self.expand_data_on_trace(&data, trace, SEL_OP_ARITH256, previous_lt_flags, cache);
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

    fn process_arith256_mod<R: ArithEqRow<F>>(
        &self,
        input: &Arith256ModInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Arith256Mod::execute(&input.a, &input.b, &input.c, &input.module);
        self.expand_data_on_trace(&data, trace, SEL_OP_ARITH256_MOD, previous_lt_flags, cache);
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
    fn process_secp256k1_add<R: ArithEqRow<F>>(
        &self,
        input: &Secp256k1AddInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Secp256k1::execute_add(&input.p1, &input.p2);
        self.expand_data_on_trace(&data, trace, SEL_OP_SECP256K1_ADD, previous_lt_flags, cache);
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
    fn process_secp256k1_dbl<R: ArithEqRow<F>>(
        &self,
        input: &Secp256k1DblInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Secp256k1::execute_dbl(&input.p1);
        self.expand_data_on_trace(&data, trace, SEL_OP_SECP256K1_DBL, previous_lt_flags, cache);
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

    fn process_bn254_curve_add<R: ArithEqRow<F>>(
        &self,
        input: &Bn254CurveAddInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Bn254Curve::execute_add(&input.p1, &input.p2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BN254_CURVE_ADD, previous_lt_flags, cache);
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

    fn process_bn254_curve_dbl<R: ArithEqRow<F>>(
        &self,
        input: &Bn254CurveDblInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Bn254Curve::execute_dbl(&input.p1);
        self.expand_data_on_trace(&data, trace, SEL_OP_BN254_CURVE_DBL, previous_lt_flags, cache);
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

    fn process_bn254_complex_add<R: ArithEqRow<F>>(
        &self,
        input: &Bn254ComplexAddInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Bn254Complex::execute_add(&input.f1, &input.f2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BN254_COMPLEX_ADD, previous_lt_flags, cache);
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

    fn process_bn254_complex_sub<R: ArithEqRow<F>>(
        &self,
        input: &Bn254ComplexSubInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Bn254Complex::execute_sub(&input.f1, &input.f2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BN254_COMPLEX_SUB, previous_lt_flags, cache);
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

    fn process_bn254_complex_mul<R: ArithEqRow<F>>(
        &self,
        input: &Bn254ComplexMulInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Bn254Complex::execute_mul(&input.f1, &input.f2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BN254_COMPLEX_MUL, previous_lt_flags, cache);
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

    fn process_secp256r1_add<R: ArithEqRow<F>>(
        &self,
        input: &Secp256r1AddInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Secp256r1::execute_add(&input.p1, &input.p2);
        self.expand_data_on_trace(&data, trace, SEL_OP_SECP256R1_ADD, previous_lt_flags, cache);
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

    fn process_secp256r1_dbl<R: ArithEqRow<F>>(
        &self,
        input: &Secp256r1DblInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Secp256r1::execute_dbl(&input.p1);
        self.expand_data_on_trace(&data, trace, SEL_OP_SECP256R1_DBL, previous_lt_flags, cache);
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

    const FIRST_CLOCK: u8 = 0;
    const LAST_CLOCK: u8 = ARITH_EQ_ROWS_BY_OP as u8 - 1;
    fn expand_data_on_trace<R: ArithEqRow<F>>(
        &self,
        data: &executors::ArithEqData,
        trace: &mut [R],
        sel_op: usize,
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let mut x1_x2_different = false;

        // To calculate the multiplicity, use the prev_x3_lt and prev_y3_lt flags. However, when
        // calculating the delta on the first clock, the prev_lt flags must always be treated as 0.
        let mut prev_x3_lt = (previous_lt_flags & 1) != 0;
        let mut prev_y3_lt = (previous_lt_flags & 2) != 0;

        #[allow(clippy::needless_range_loop)]
        for i in 0..ARITH_EQ_ROWS_BY_OP {
            // Compute all carry values first
            // Only range-check/fill the carry rows this config actually has (MAX_CEQS); a reduced
            // air range-checking all 3 would over-contribute to the carry range-check bus.
            let mut carry_values = [[0u64; 2]; 3];
            for j in 0..R::CEQS {
                // first position without carry
                let carry_0 = if i == 0 { 0 } else { data.cout[i * 2 - 1][j] };
                carry_values[j][0] = to_field::<F>(cache.carry(carry_0));
                carry_values[j][1] = to_field::<F>(cache.carry(data.cout[i * 2][j]));
            }
            trace[i].set_carry(&carry_values);

            let q_last_clock = i == ARITH_EQ_ROWS_BY_OP - 1;
            trace[i].set_x1(to_field::<F>(cache.chunk(data.x1[i])) as u16);
            trace[i].set_y1(to_field::<F>(cache.chunk(data.y1[i])) as u16);
            trace[i].set_x2(to_field::<F>(cache.chunk(data.x2[i])) as u16);
            trace[i].set_y2(to_field::<F>(cache.chunk(data.y2[i])) as u16);
            trace[i].set_x3(to_field::<F>(cache.chunk(data.x3[i])) as u16);
            trace[i].set_y3(to_field::<F>(cache.chunk(data.y3[i])) as u16);
            // Quotients / lambda: range-check + fill only the columns this config has, so the shared
            // witness registers exactly the std range-checks the config's PIL looks up.
            if R::QS >= 1 {
                trace[i].set_q0(to_field::<F>(cache.q_column(data.q0[i], q_last_clock)) as u32);
            }
            if R::QS >= 2 {
                trace[i].set_q1(to_field::<F>(cache.q_column(data.q1[i], q_last_clock)) as u32);
            }
            if R::QS >= 3 {
                trace[i].set_q2(to_field::<F>(cache.q_column(data.q2[i], q_last_clock)) as u32);
            }
            if R::USE_S {
                trace[i].set_s(to_field::<F>(cache.chunk(data.s[i])) as u32);
            }

            // Set the one-hot operation selector (and its clk0 twin on the first clock). Iterating
            // all ops mirrors the previous full-array write, so it doesn't rely on zero-init; ops the
            // active config doesn't have resolve to no-op arms in the row impl.
            let active_op = ArithEqOp::ALL[sel_op];
            for op in ArithEqOp::ALL {
                trace[i].set_sel(op, op == active_op);
                trace[i].set_sel_clk0(op, i == 0 && op == active_op);
            }
            let iclock = match i as u8 {
                Self::FIRST_CLOCK => 1,
                Self::LAST_CLOCK => 2,
                _ => 0,
            };
            match sel_op {
                SEL_OP_ARITH256_MOD => {
                    let x3_lt = data.x3[i] < data.y2[i]
                        || (i > 0 && data.x3[i] == data.y2[i] && prev_x3_lt);
                    trace[i].set_x3_lt(x3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_x3_lt,
                        x3_lt,
                        data.x3[i] - data.y2[i],
                        iclock,
                    );
                    cache.lt_row(row);
                    prev_x3_lt = x3_lt;

                    trace[i].set_y3_lt(false);
                }
                SEL_OP_SECP256K1_ADD | SEL_OP_SECP256K1_DBL => {
                    let x3_lt = data.x3[i] < SECP256K1_PRIME_CHUNKS[i]
                        || (i > 0 && data.x3[i] == SECP256K1_PRIME_CHUNKS[i] && prev_x3_lt);
                    trace[i].set_x3_lt(x3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_x3_lt,
                        x3_lt,
                        data.x3[i] - SECP256K1_PRIME_CHUNKS[i],
                        iclock,
                    );
                    cache.lt_row(row);
                    prev_x3_lt = x3_lt;

                    let y3_lt = data.y3[i] < SECP256K1_PRIME_CHUNKS[i]
                        || (i > 0 && data.y3[i] == SECP256K1_PRIME_CHUNKS[i] && prev_y3_lt);
                    trace[i].set_y3_lt(y3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_y3_lt,
                        y3_lt,
                        data.y3[i] - SECP256K1_PRIME_CHUNKS[i],
                        iclock,
                    );
                    cache.lt_row(row);
                    prev_y3_lt = y3_lt;
                }
                SEL_OP_BN254_CURVE_ADD
                | SEL_OP_BN254_CURVE_DBL
                | SEL_OP_BN254_COMPLEX_ADD
                | SEL_OP_BN254_COMPLEX_SUB
                | SEL_OP_BN254_COMPLEX_MUL => {
                    let x3_lt = data.x3[i] < BN254_PRIME_CHUNKS[i]
                        || (i > 0 && data.x3[i] == BN254_PRIME_CHUNKS[i] && prev_x3_lt);
                    trace[i].set_x3_lt(x3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_x3_lt,
                        x3_lt,
                        data.x3[i] - BN254_PRIME_CHUNKS[i],
                        iclock,
                    );
                    cache.lt_row(row);
                    prev_x3_lt = x3_lt;

                    let y3_lt = data.y3[i] < BN254_PRIME_CHUNKS[i]
                        || (i > 0 && data.y3[i] == BN254_PRIME_CHUNKS[i] && prev_y3_lt);
                    trace[i].set_y3_lt(y3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_y3_lt,
                        y3_lt,
                        data.y3[i] - BN254_PRIME_CHUNKS[i],
                        iclock,
                    );
                    cache.lt_row(row);
                    prev_y3_lt = y3_lt;
                }
                SEL_OP_SECP256R1_ADD | SEL_OP_SECP256R1_DBL => {
                    let x3_lt = data.x3[i] < SECP256R1_PRIME_CHUNKS[i]
                        || (i > 0 && data.x3[i] == SECP256R1_PRIME_CHUNKS[i] && prev_x3_lt);
                    trace[i].set_x3_lt(x3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_x3_lt,
                        x3_lt,
                        data.x3[i] - SECP256R1_PRIME_CHUNKS[i],
                        iclock,
                    );
                    cache.lt_row(row);
                    prev_x3_lt = x3_lt;

                    let y3_lt = data.y3[i] < SECP256R1_PRIME_CHUNKS[i]
                        || (i > 0 && data.y3[i] == SECP256R1_PRIME_CHUNKS[i] && prev_y3_lt);
                    trace[i].set_y3_lt(y3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_y3_lt,
                        y3_lt,
                        data.y3[i] - SECP256R1_PRIME_CHUNKS[i],
                        iclock,
                    );
                    cache.lt_row(row);
                    prev_y3_lt = y3_lt;
                }
                _ => {
                    trace[i].set_x3_lt(false);
                    trace[i].set_y3_lt(false);
                }
            }
            if (sel_op == SEL_OP_SECP256K1_ADD)
                || (sel_op == SEL_OP_BN254_CURVE_ADD)
                || (sel_op == SEL_OP_SECP256R1_ADD)
            {
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

    /// Writes one operation's rows. Split out of the fill so the batched walk and the dispatch
    /// stay separate concerns; the body is the match the per-operation closure used to hold.
    fn process_input<R: ArithEqRow<F>>(
        &self,
        input: &ArithEqInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        match input {
            ArithEqInput::Arith256(idata) => {
                self.process_arith256(idata, trace, previous_lt_flags, cache)
            }
            ArithEqInput::Arith256Mod(idata) => {
                self.process_arith256_mod(idata, trace, previous_lt_flags, cache)
            }
            ArithEqInput::Secp256k1Add(idata) => {
                self.process_secp256k1_add(idata, trace, previous_lt_flags, cache)
            }
            ArithEqInput::Secp256k1Dbl(idata) => {
                self.process_secp256k1_dbl(idata, trace, previous_lt_flags, cache)
            }
            ArithEqInput::Bn254CurveAdd(idata) => {
                self.process_bn254_curve_add(idata, trace, previous_lt_flags, cache)
            }
            ArithEqInput::Bn254CurveDbl(idata) => {
                self.process_bn254_curve_dbl(idata, trace, previous_lt_flags, cache)
            }
            ArithEqInput::Bn254ComplexAdd(idata) => {
                self.process_bn254_complex_add(idata, trace, previous_lt_flags, cache);
            }
            ArithEqInput::Bn254ComplexSub(idata) => {
                self.process_bn254_complex_sub(idata, trace, previous_lt_flags, cache);
            }
            ArithEqInput::Bn254ComplexMul(idata) => {
                self.process_bn254_complex_mul(idata, trace, previous_lt_flags, cache);
            }
            ArithEqInput::Secp256r1Add(idata) => {
                self.process_secp256r1_add(idata, trace, previous_lt_flags, cache)
            }
            ArithEqInput::Secp256r1Dbl(idata) => {
                self.process_secp256r1_dbl(idata, trace, previous_lt_flags, cache)
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
    pub fn compute_witness<R: ArithEqRow<F> + Sync>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<ArithEqInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        // The concrete trace (and thus the resulting `AirInstance`'s `AIR_ID`) is selected by the row
        // type `R` via its `ArithEqRow::Trace`, so this one body serves every config.
        let mut trace = R::new_trace(trace_buffer)?;
        let num_rows = R::trace_num_rows(&trace);
        let total_inputs: usize = inputs.iter().map(|x| x.len()).sum();
        let num_rows_needed = total_inputs * ARITH_EQ_ROWS_BY_OP;

        tracing::debug!(
            "··· Creating {} instance [{} / {} rows filled {:.2}%]",
            R::AIR_NAME,
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );
        let full = num_rows_needed == num_rows;

        timer_start_trace!(ARITH_EQ_TRACE);

        phase_start!(t_prep);
        // One batch per thread of the pool this witness computation already runs in, rather than one
        // rayon task per operation. The operations are laid out in order, `ARITH_EQ_ROWS_BY_OP` rows
        // each, so a batch is a contiguous run of rows whose operations are a contiguous run of the
        // input sequence -- no per-operation bookkeeping vector, and no sequential pass to build it.
        //
        // What used to force that pass is `previous_lt_flags`, which chains from each operation to
        // the next. It is a pure function of the preceding operation, so a batch works out its own
        // starting value in O(1) rather than inheriting it from a walk over everything before it.
        let n_batches = rayon::current_num_threads().max(1);
        let ops_per_batch = total_inputs.div_ceil(n_batches).max(1);

        // Cumulative operation count per input chunk: lets a batch find its first operation by
        // binary search instead of walking the ones before it, and keeps the chunks unconcatenated.
        let mut chunk_start: Vec<usize> = Vec::with_capacity(inputs.len() + 1);
        chunk_start.push(0);
        for chunk in inputs {
            chunk_start.push(chunk_start[chunk_start.len() - 1] + chunk.len());
        }

        // A full instance wraps around: the first operation's predecessor is the last one.
        let last_flags =
            inputs.iter().rev().find_map(|c| c.last()).map(Self::get_lt_flags).unwrap_or(0);

        let index = total_inputs;
        phase_end!(d_prep, t_prep);
        phase_max_start!(init_max);
        phase_start!(t_fill);

        // Each batch counts its range checks into its own cache instead of atomically into the
        // shared table, and returns it. `init` is timed apart because a cache is 48 MiB: `vec![0; _]`
        // gets zeroed pages from the kernel, so the cost is the batch faulting in the pages it
        // actually touches rather than a memset up front.
        let caches: Vec<MultiplicityCache> = R::trace_rows(&mut trace)[..num_rows_needed]
            .par_chunks_mut(ops_per_batch * ARITH_EQ_ROWS_BY_OP)
            .enumerate()
            .map(|(batch, batch_rows)| {
                phase_start!(t_init);
                let mut cache = MultiplicityCache::new();
                phase_max_record!(init_max, t_init);

                let first_op = batch * ops_per_batch;
                let mut previous_lt_flags = if first_op == 0 {
                    if full {
                        last_flags
                    } else {
                        0
                    }
                } else {
                    Self::get_lt_flags(op_at(inputs, &chunk_start, first_op - 1))
                };
                for (input, rows) in ops_from(inputs, &chunk_start, first_op)
                    .zip(batch_rows.chunks_mut(ARITH_EQ_ROWS_BY_OP))
                {
                    self.process_input(input, rows, previous_lt_flags, &mut cache);
                    previous_lt_flags = Self::get_lt_flags(input);
                }
                cache
            })
            .collect();

        phase_end!(d_fill, t_fill);

        phase_start!(t_merge);
        let mut caches = caches.into_iter();
        let merged = caches.next().map(|mut first| {
            for other in caches {
                first.add(&other);
            }
            first
        });

        phase_end!(d_merge, t_merge);

        phase_start!(t_flush);
        if let Some(merged) = &merged {
            merged.flush(
                &self.std,
                self.q_hsc_range_id,
                self.chunk_range_id,
                self.carry_range_id,
                self.table_id,
            );
        }

        phase_end!(d_flush, t_flush);

        phase_start!(t_pad);

        // Padding range-checks per unused op-slot, derived from this config's column set so they
        // match the PIL exactly (full air: QS=3, USE_S=true, CEQS=3 → 3 / 157 / 96):
        //   q_hsc: QS q-columns range-checked on the last clock only            → QS
        //   chunk: x1..y3 (6·16=96) + q on the 15 non-last clocks (QS·15) + s (USE_S·16)
        //   carry: MAX_CEQS · CBC(2) · 16 rows                                   → CEQS·32
        // Capacity of *this* config's air, taken from the trace: the configs come in two heights
        // and hold a different number of operations each.
        let padding_ops = (num_rows / ARITH_EQ_ROWS_BY_OP - index) as u64;
        let q_hsc_per_op = R::QS as u64;
        let chunk_per_op = 96 + R::QS as u64 * 15 + if R::USE_S { 16 } else { 0 };
        let carry_per_op = R::CEQS as u64 * 32;
        self.std.range_check(self.q_hsc_range_id, 0, q_hsc_per_op * padding_ops);
        self.std.range_check(self.chunk_range_id, 0, chunk_per_op * padding_ops);
        self.std.range_check(self.carry_range_id, 0, carry_per_op * padding_ops);

        let padding_row = R::default();

        R::trace_rows(&mut trace)[num_rows_needed..num_rows]
            .par_iter_mut()
            .for_each(|slot| *slot = padding_row);

        phase_end!(d_pad, t_pad);
        phase_log!(
            "{} witness: {} ops, {} threads, {} caches x {:.1}MiB | prep {:.1}ms \
             cache init {:.1}ms (slowest batch) fill {:.0}ms merge {:.0}ms flush {:.0}ms pad {:.0}ms",
            R::AIR_NAME,
            total_inputs,
            rayon::current_num_threads(),
            n_batches,
            CACHE_BYTES as f64 / (1024.0 * 1024.0),
            phase_ms!(d_prep),
            phase_max_ms!(init_max),
            phase_ms!(d_fill),
            phase_ms!(d_merge),
            phase_ms!(d_flush),
            phase_ms!(d_pad)
        );
        timer_stop_and_log_trace!(ARITH_EQ_TRACE);

        Ok(R::into_air_instance(&mut trace))
    }
}

/// The operation at flat index `g`, located through the per-chunk cumulative offsets.
fn op_at<'a>(inputs: &'a [Vec<ArithEqInput>], chunk_start: &[usize], g: usize) -> &'a ArithEqInput {
    let chunk = chunk_start.partition_point(|&start| start <= g) - 1;
    &inputs[chunk][g - chunk_start[chunk]]
}

/// The operations from flat index `g` onwards, in order, chaining the chunks lazily rather than
/// concatenating them.
fn ops_from<'a>(
    inputs: &'a [Vec<ArithEqInput>],
    chunk_start: &[usize],
    g: usize,
) -> impl Iterator<Item = &'a ArithEqInput> {
    let chunk = chunk_start.partition_point(|&start| start <= g) - 1;
    let offset = g - chunk_start[chunk];
    inputs[chunk][offset..].iter().chain(inputs[chunk + 1..].iter().flat_map(|c| c.iter()))
}

/// A chunk value as a field element. Split out of the old `to_ranged_field`, whose other half (the
/// range-check bookkeeping) now goes to the batch's [`MultiplicityCache`].
#[inline(always)]
fn to_field<F: PrimeField64>(value: i64) -> u64 {
    if value >= 0 {
        value as u64
    } else {
        (F::ORDER_U64 as i64 + value) as u64
    }
}
