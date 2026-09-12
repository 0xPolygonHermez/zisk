use proofman_fields::PrimeField64;
// `phase_ms` / `phase_max_ms` are only read inside a `phase_log!`, which vanishes without the
// `witness_timers` feature -- and takes the only use of those imports with it.
use rayon::prelude::*;
use std::sync::Arc;
#[allow(unused_imports)]
use zisk_common::{
    phase_end, phase_log, phase_max_ms, phase_max_record, phase_max_start, phase_ms, phase_start,
};

use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, GenericTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::{ArithEq384TraceRowOps, ZISK_AIRGROUP_ID};
use zisk_precomp_arith_eq::ArithEqLtTableSM;
// `CACHE_BYTES` is only reported by the `witness_timers` line.
#[allow(unused_imports)]
use zisk_precomp_common::{MultiplicityCache, CACHE_BYTES};

use crate::slope_inverses::{SlopeInverses, INV_RUN_OPS};

use crate::{
    arith_eq_384_constants::*, executors, Arith384ModInput, ArithEq384Input,
    Bls12_381ComplexAddInput, Bls12_381ComplexMulInput, Bls12_381ComplexSubInput,
    Bls12_381CurveAddInput, Bls12_381CurveDblInput,
};

/// The `ArithEq384SM` struct encapsulates the logic of the ArithEq384 State Machine.
///
/// Nothing here depends on the height of the air: the same state machine serves every rung of the
/// ladder (`ArithEq384` at `2**20`, `ArithEq384Large` at `2**22`, `ArithEq384Huge` at `2**23`), and
/// the capacity is taken from the trace each call builds.
pub struct ArithEq384SM<F: PrimeField64> {
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
        let q_hsc_range_id =
            std.get_range_id(0, ARITH_EQ_384_Q_HSC_MAX, None).expect("Failed to get range ID");
        let chunk_range_id = std
            .get_range_id(0, ARITH_EQ_384_CHUNK_MAX as i64, None)
            .expect("Failed to get range ID");
        let carry_range_id = std
            .get_range_id(ARITH_EQ_384_CARRY_MIN, ARITH_EQ_384_CARRY_MAX, None)
            .expect("Failed to get range ID");

        // Get the table ID
        let table_id =
            std.get_virtual_table_id(ArithEqLtTableSM::TABLE_ID).expect("Failed to get table ID");

        Arc::new(Self { std, q_hsc_range_id, chunk_range_id, carry_range_id, table_id })
    }
    // Returns the LT flags for x3 and y3. The flags are determined solely by the operation type.
    /// Writes one operation's rows. Split out of the fill so the batched walk and the dispatch stay
    /// separate concerns; the body is the match the per-operation closure used to hold.
    fn process_input<R: ArithEq384TraceRowOps<F>>(
        &self,
        input: &ArithEq384Input,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
        inverses: &mut SlopeInverses,
    ) {
        match input {
            ArithEq384Input::Arith384Mod(idata) => {
                self.process_arith384_mod(idata, trace, previous_lt_flags, cache)
            }
            ArithEq384Input::Bls12_381CurveAdd(idata) => {
                let den_inv = inverses.next_inverse();
                self.process_bls12_381_curve_add(idata, den_inv, trace, previous_lt_flags, cache)
            }
            ArithEq384Input::Bls12_381CurveDbl(idata) => {
                let den_inv = inverses.next_inverse();
                self.process_bls12_381_curve_dbl(idata, den_inv, trace, previous_lt_flags, cache)
            }
            ArithEq384Input::Bls12_381ComplexAdd(idata) => {
                self.process_bls12_381_complex_add(idata, trace, previous_lt_flags, cache);
            }
            ArithEq384Input::Bls12_381ComplexSub(idata) => {
                self.process_bls12_381_complex_sub(idata, trace, previous_lt_flags, cache);
            }
            ArithEq384Input::Bls12_381ComplexMul(idata) => {
                self.process_bls12_381_complex_mul(idata, trace, previous_lt_flags, cache);
            }
        }
    }

    fn get_lt_flags(input: &ArithEq384Input) -> u8 {
        const X3_LT_FLAG: u8 = 1;
        const Y3_LT_FLAG: u8 = 2;

        match input {
            ArithEq384Input::Arith384Mod(_) => X3_LT_FLAG,
            ArithEq384Input::Bls12_381CurveAdd(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEq384Input::Bls12_381CurveDbl(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEq384Input::Bls12_381ComplexAdd(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEq384Input::Bls12_381ComplexSub(_) => X3_LT_FLAG | Y3_LT_FLAG,
            ArithEq384Input::Bls12_381ComplexMul(_) => X3_LT_FLAG | Y3_LT_FLAG,
        }
    }
    fn expand_addr_step_on_trace<R: ArithEq384TraceRowOps<F>>(
        data: &ArithEq384StepAddr,
        trace: &mut [R],
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

    fn process_arith384_mod<R: ArithEq384TraceRowOps<F>>(
        &self,
        input: &Arith384ModInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Arith384Mod::execute(&input.a, &input.b, &input.c, &input.module);
        self.expand_data_on_trace(&data, trace, SEL_OP_ARITH384_MOD, previous_lt_flags, cache);
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

    /// `den_inv` is this operation's slope denominator already inverted, taken from the run's
    /// batch inversion -- see [`crate::slope_inverses`].
    fn process_bls12_381_curve_add<R: ArithEq384TraceRowOps<F>>(
        &self,
        input: &Bls12_381CurveAddInput,
        den_inv: executors::Bls12_381Field,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Bls12_381Curve::execute_add_dbl_with_inv(
            false, &input.p1, &input.p2, den_inv,
        );
        self.expand_data_on_trace(
            &data,
            trace,
            SEL_OP_BLS12_381_CURVE_ADD,
            previous_lt_flags,
            cache,
        );
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

    /// `den_inv` is this operation's slope denominator already inverted, taken from the run's
    /// batch inversion -- see [`crate::slope_inverses`].
    fn process_bls12_381_curve_dbl<R: ArithEq384TraceRowOps<F>>(
        &self,
        input: &Bls12_381CurveDblInput,
        den_inv: executors::Bls12_381Field,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Bls12_381Curve::execute_add_dbl_with_inv(
            true, &input.p1, &input.p1, den_inv,
        );
        self.expand_data_on_trace(
            &data,
            trace,
            SEL_OP_BLS12_381_CURVE_DBL,
            previous_lt_flags,
            cache,
        );
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

    fn process_bls12_381_complex_add<R: ArithEq384TraceRowOps<F>>(
        &self,
        input: &Bls12_381ComplexAddInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Bls12_381Complex::execute_add(&input.f1, &input.f2);
        self.expand_data_on_trace(
            &data,
            trace,
            SEL_OP_BLS12_381_COMPLEX_ADD,
            previous_lt_flags,
            cache,
        );
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

    fn process_bls12_381_complex_sub<R: ArithEq384TraceRowOps<F>>(
        &self,
        input: &Bls12_381ComplexSubInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Bls12_381Complex::execute_sub(&input.f1, &input.f2);
        self.expand_data_on_trace(
            &data,
            trace,
            SEL_OP_BLS12_381_COMPLEX_SUB,
            previous_lt_flags,
            cache,
        );
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

    fn process_bls12_381_complex_mul<R: ArithEq384TraceRowOps<F>>(
        &self,
        input: &Bls12_381ComplexMulInput,
        trace: &mut [R],
        previous_lt_flags: u8,
        cache: &mut MultiplicityCache,
    ) {
        let data = executors::Bls12_381Complex::execute_mul(&input.f1, &input.f2);
        self.expand_data_on_trace(
            &data,
            trace,
            SEL_OP_BLS12_381_COMPLEX_MUL,
            previous_lt_flags,
            cache,
        );
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

    const FIRST_CLOCK: u8 = 0;
    const LAST_CLOCK: u8 = ARITH_EQ_384_ROWS_BY_OP as u8 - 1;

    fn expand_data_on_trace<R: ArithEq384TraceRowOps<F>>(
        &self,
        data: &executors::ArithEq384Data,
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
        for i in 0..ARITH_EQ_384_ROWS_BY_OP {
            let mut carry_values = [[0u64; 2]; 3];
            for j in 0..3 {
                // first position without carry
                let carry_0 = if i == 0 { 0 } else { data.cout[i * 2 - 1][j] };
                carry_values[j][0] = to_field::<F>(cache.carry(carry_0));
                carry_values[j][1] = to_field::<F>(cache.carry(data.cout[i * 2][j]));
            }
            trace[i].set_all_carry(&carry_values);
            let q_last_clock = i == ARITH_EQ_384_ROWS_BY_OP - 1;
            trace[i].set_x1(to_field::<F>(cache.chunk(data.x1[i])) as u16);
            trace[i].set_y1(to_field::<F>(cache.chunk(data.y1[i])) as u16);
            trace[i].set_x2(to_field::<F>(cache.chunk(data.x2[i])) as u16);
            trace[i].set_y2(to_field::<F>(cache.chunk(data.y2[i])) as u16);
            trace[i].set_x3(to_field::<F>(cache.chunk(data.x3[i])) as u16);
            trace[i].set_y3(to_field::<F>(cache.chunk(data.y3[i])) as u16);
            trace[i].set_q0(to_field::<F>(cache.q_column(data.q0[i], q_last_clock)) as u32);
            trace[i].set_q1(to_field::<F>(cache.q_column(data.q1[i], q_last_clock)) as u32);
            trace[i].set_q2(to_field::<F>(cache.q_column(data.q2[i], q_last_clock)) as u32);
            trace[i].set_s(to_field::<F>(cache.chunk(data.s[i])) as u32);

            // TODO Range check
            // Compute sel_op arrays
            let mut sel_op_values = [false; ARITH_EQ_384_OP_NUM];
            sel_op_values[sel_op] = true;
            trace[i].set_all_sel_op(&sel_op_values);
            let iclock = match i as u8 {
                Self::FIRST_CLOCK => 1,
                Self::LAST_CLOCK => 2,
                _ => 0,
            };
            match sel_op {
                SEL_OP_ARITH384_MOD => {
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
                SEL_OP_BLS12_381_CURVE_ADD
                | SEL_OP_BLS12_381_CURVE_DBL
                | SEL_OP_BLS12_381_COMPLEX_ADD
                | SEL_OP_BLS12_381_COMPLEX_SUB
                | SEL_OP_BLS12_381_COMPLEX_MUL => {
                    let x3_lt = data.x3[i] < BLS12_381_PRIME_CHUNKS[i]
                        || (i > 0 && data.x3[i] == BLS12_381_PRIME_CHUNKS[i] && prev_x3_lt);
                    trace[i].set_x3_lt(x3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_x3_lt,
                        x3_lt,
                        data.x3[i] - BLS12_381_PRIME_CHUNKS[i],
                        iclock,
                    );
                    cache.lt_row(row);
                    prev_x3_lt = x3_lt;

                    let y3_lt = data.y3[i] < BLS12_381_PRIME_CHUNKS[i]
                        || (i > 0 && data.y3[i] == BLS12_381_PRIME_CHUNKS[i] && prev_y3_lt);
                    trace[i].set_y3_lt(y3_lt);
                    let row = ArithEqLtTableSM::calculate_table_row(
                        prev_y3_lt,
                        y3_lt,
                        data.y3[i] - BLS12_381_PRIME_CHUNKS[i],
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
    /// The air is selected by the `NUM_ROWS` / `AIR_ID` consts of the trace this builds, so one
    /// body serves every height the air is instantiated at.
    pub fn compute_witness<
        R: ArithEq384TraceRowOps<F>,
        const NUM_ROWS: usize,
        const AIR_ID: usize,
    >(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<ArithEq384Input>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = GenericTrace::<R, NUM_ROWS, ZISK_AIRGROUP_ID, AIR_ID>::new_from_vec_zeroes(
            trace_buffer,
        )?;
        let num_rows = trace.num_rows();
        // Capacity of *this* air, not of the shortest one: the two heights hold a different number
        // of operations, and a `Large` instance priced against the short air's capacity would reject
        // the very inputs the planner routed to it.
        let num_available_ops = arith_eq_384_ops_per_instance(num_rows);
        let num_non_usable_rows = (num_rows % ARITH_EQ_384_ROWS_BY_OP) as u64;

        let total_inputs: usize = inputs.iter().map(|x| x.len()).sum();
        let all_ops_used = total_inputs == num_available_ops;
        let num_rows_needed = if total_inputs < num_available_ops {
            total_inputs * ARITH_EQ_384_ROWS_BY_OP
        } else if all_ops_used {
            num_rows
        } else {
            panic!(
                "Exceeded available ArithEq384 inputs: requested {}, but only {} are available.",
                total_inputs, num_available_ops
            );
        };

        tracing::debug!(
            "··· Creating ArithEq384 instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );
        timer_start_trace!(ARITH_EQ_384_TRACE);

        // One batch per thread of the pool this witness computation already runs in, rather than one
        // rayon task per operation, and each batch counts its range checks into its own cache
        // instead of atomically into the shared table. Same shape as `ArithEq`: the operations are
        // laid out in order, `ARITH_EQ_384_ROWS_BY_OP` rows each, and `previous_lt_flags` is a pure
        // function of the preceding operation, so a batch resolves its starting value in O(1).
        let n_batches = rayon::current_num_threads().max(1);
        let ops_per_batch = total_inputs.div_ceil(n_batches).max(1);

        // Cumulative operation count per input chunk, so a batch finds its first operation by binary
        // search rather than walking the ones before it, and the chunks stay unconcatenated.
        let mut chunk_start: Vec<usize> = Vec::with_capacity(inputs.len() + 1);
        chunk_start.push(0);
        for chunk in inputs {
            chunk_start.push(chunk_start[chunk_start.len() - 1] + chunk.len());
        }

        // NOTE: unlike `ArithEq`, this air does not wrap the flags around on a full instance --
        // the sequential fill this replaces started the first operation at 0 unconditionally, and
        // its constraints are written for that.

        let index = total_inputs;
        phase_max_start!(init_max);
        phase_start!(t_fill);

        // Only the rows the operations occupy, NOT `num_rows_needed`: when the instance is exactly
        // full that is the air's whole height, and the height is not a multiple of
        // `ARITH_EQ_384_ROWS_BY_OP` (2^20 leaves 16 rows over). Those trailing rows are padding, and
        // slicing them in here would open one batch past the last operation.
        let fill_rows = total_inputs * ARITH_EQ_384_ROWS_BY_OP;
        let caches: Vec<MultiplicityCache> = trace.buffer[..fill_rows]
            .par_chunks_mut(ops_per_batch * ARITH_EQ_384_ROWS_BY_OP)
            .enumerate()
            .map(|(batch, batch_rows)| {
                phase_start!(t_init);
                let mut cache = MultiplicityCache::new();
                phase_max_record!(init_max, t_init);

                let first_op = batch * ops_per_batch;
                let mut previous_lt_flags = if first_op == 0 {
                    0
                } else {
                    Self::get_lt_flags(op_at(inputs, &chunk_start, first_op - 1))
                };
                // The batch is filled a run at a time, each run seeing its operations twice:
                // once to collect their slope denominators, which are inverted together, and once
                // to write their rows. See [`crate::slope_inverses`] for what that buys.
                let mut ops = ops_from(inputs, &chunk_start, first_op);
                let mut inverses = SlopeInverses::default();
                for run_rows in batch_rows.chunks_mut(INV_RUN_OPS * ARITH_EQ_384_ROWS_BY_OP) {
                    inverses.reload(ops.clone().take(run_rows.len() / ARITH_EQ_384_ROWS_BY_OP));
                    for rows in run_rows.chunks_mut(ARITH_EQ_384_ROWS_BY_OP) {
                        // The two passes must agree operation for operation: this is the same
                        // sequence `reload` just walked, taken one at a time.
                        let input = ops.next().unwrap();
                        self.process_input(
                            input,
                            rows,
                            previous_lt_flags,
                            &mut cache,
                            &mut inverses,
                        );
                        previous_lt_flags = Self::get_lt_flags(input);
                    }
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
        phase_log!(
            "ArithEq384 witness: {} ops, {} threads, {} caches x {:.1}MiB | cache init {:.1}ms \
             (slowest batch) fill {:.0}ms merge {:.0}ms flush {:.0}ms",
            total_inputs,
            rayon::current_num_threads(),
            n_batches,
            CACHE_BYTES as f64 / (1024.0 * 1024.0),
            init_max.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e3,
            phase_ms!(d_fill),
            phase_ms!(d_merge),
            phase_ms!(d_flush)
        );

        // Padding

        // All-zero padding rows satisfy every constraint, so we must only handle the range_checks.

        let padding_ops = (num_available_ops - index) as u64;
        let q_hsc_range_mult = 3 * padding_ops;
        let chunk_range_mult = (7 * ARITH_EQ_384_ROWS_BY_OP as u64
            + 3 * (ARITH_EQ_384_ROWS_BY_OP - 1) as u64)
            * padding_ops
            + 10 * num_non_usable_rows; // 7 chunk_cols + 3 q_cols on every tail row
        let carry_range_mult =
            (6 * ARITH_EQ_384_ROWS_BY_OP as u64) * padding_ops + 6 * num_non_usable_rows; // 6 carry_cols
        self.std.range_check(self.q_hsc_range_id, 0, q_hsc_range_mult);
        self.std.range_check(self.chunk_range_id, 0, chunk_range_mult);
        self.std.range_check(self.carry_range_id, 0, carry_range_mult);

        timer_stop_and_log_trace!(ARITH_EQ_384_TRACE);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}

/// The operation at flat index `g`, located through the per-chunk cumulative offsets.
fn op_at<'a>(
    inputs: &'a [Vec<ArithEq384Input>],
    chunk_start: &[usize],
    g: usize,
) -> &'a ArithEq384Input {
    let chunk = chunk_start.partition_point(|&start| start <= g) - 1;
    &inputs[chunk][g - chunk_start[chunk]]
}

/// The operations from flat index `g` onwards, in order, chaining the chunks lazily rather than
/// concatenating them.
fn ops_from<'a>(
    inputs: &'a [Vec<ArithEq384Input>],
    chunk_start: &[usize],
    g: usize,
) -> impl Iterator<Item = &'a ArithEq384Input> + Clone {
    let chunk = chunk_start.partition_point(|&start| start <= g) - 1;
    let offset = g - chunk_start[chunk];
    inputs[chunk][offset..].iter().chain(inputs[chunk + 1..].iter().flat_map(|c| c.iter()))
}

/// A chunk value as a field element. The other half of the old `to_ranged_field` (the range-check
/// bookkeeping) now goes to the batch's [`MultiplicityCache`].
#[inline(always)]
fn to_field<F: PrimeField64>(value: i64) -> u64 {
    if value >= 0 {
        value as u64
    } else {
        (F::ORDER_U64 as i64 + value) as u64
    }
}
