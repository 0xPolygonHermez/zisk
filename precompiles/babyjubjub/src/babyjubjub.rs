use fields::PrimeField64;
use std::sync::Arc;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::{BabyJubJubTrace, BabyJubJubTraceRowOps};

use crate::{
    executors, BabyJubJubAddInput, BabyJubJubInput, BabyJubJubLtTableSM, BABYJUBJUB_OP_NUM,
    BABYJUBJUB_PRIME_CHUNKS, BABYJUBJUB_ROWS_BY_OP, SEL_OP_BABYJUBJUB_ADD,
};
use rayon::prelude::*;

/// The `BabyJubJubSM` struct encapsulates the logic of the BabyJubJub State Machine.
pub struct BabyJubJubSM<F: PrimeField64> {
    /// Number of available babyjubjub operations in the trace.
    pub num_available_ops: usize,

    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// The table ID for the BabyJubJub Lt Table State Machine.
    table_id: usize,

    pub q_hsc_range_id: usize,
    pub chunk_range_id: usize,
    pub carry_range_id: usize,
}

#[derive(Debug, Default)]
struct BabyJubJubStepAddr {
    main_step: u64,
    addr_op: u32,
    addr_x1: u32,
    addr_y1: u32,
    addr_x2: u32,
    addr_y2: u32,
    addr_x3: u32,
    addr_y3: u32,
    addr_ind: [u32; 2],
}

impl<F: PrimeField64> BabyJubJubSM<F> {
    /// Creates a new BabyJubJub State Machine instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let num_available_ops = BabyJubJubTrace::<()>::NUM_ROWS / BABYJUBJUB_ROWS_BY_OP;
        let p2_22 = 1 << 22;
        let q_hsc_range_id = std.get_range_id(0, p2_22 - 1, None).expect("Failed to get range ID");
        let chunk_range_id = std.get_range_id(0, 0xFFFF, None).expect("Failed to get range ID");
        let carry_range_id =
            std.get_range_id(-(p2_22 - 1), p2_22, None).expect("Failed to get range ID");

        let table_id = std
            .get_virtual_table_id(BabyJubJubLtTableSM::TABLE_ID)
            .expect("Failed to get table ID");

        Arc::new(Self {
            std,
            num_available_ops,
            q_hsc_range_id,
            chunk_range_id,
            carry_range_id,
            table_id,
        })
    }

    fn expand_addr_step_on_trace<R: BabyJubJubTraceRowOps<F>>(
        data: &BabyJubJubStepAddr,
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
        for i in 0..(BABYJUBJUB_ROWS_BY_OP - 8 - data.addr_ind.len()) {
            trace[i + 8 + data.addr_ind.len()].set_step_addr(0);
        }
    }

    fn process_babyjubjub_add<R: BabyJubJubTraceRowOps<F>>(
        &self,
        input: &BabyJubJubAddInput,
        trace: &mut [R],
        previous_lt_flags: u8,
    ) {
        let data = executors::BabyJubJub::execute_add(&input.p1, &input.p2);
        self.expand_data_on_trace(&data, trace, SEL_OP_BABYJUBJUB_ADD, previous_lt_flags);
        // Result overwrites p1 (same memory map as bn254 curve add).
        Self::expand_addr_step_on_trace(
            &BabyJubJubStepAddr {
                main_step: input.step,
                addr_op: input.addr,
                addr_x1: input.p1_addr,
                addr_y1: input.p1_addr + 32,
                addr_x2: input.p2_addr,
                addr_y2: input.p2_addr + 32,
                addr_x3: input.p1_addr,
                addr_y3: input.p1_addr + 32,
                addr_ind: [input.p1_addr, input.p2_addr],
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

    const FIRST_CLOCK: u8 = 0;
    const LAST_CLOCK: u8 = BABYJUBJUB_ROWS_BY_OP as u8 - 1;

    fn expand_data_on_trace<R: BabyJubJubTraceRowOps<F>>(
        &self,
        data: &executors::BabyJubJubData,
        trace: &mut [R],
        sel_op: usize,
        previous_lt_flags: u8,
    ) {
        // The complete twisted-Edwards addition always range-checks both result coordinates,
        // so the previous-row x3_lt / y3_lt flags are always meaningful (no NO_FLAGS op).
        let mut prev_x3_lt = (previous_lt_flags & 1) != 0;
        let mut prev_y3_lt = (previous_lt_flags & 2) != 0;

        #[allow(clippy::needless_range_loop)]
        for i in 0..BABYJUBJUB_ROWS_BY_OP {
            for j in 0..7 {
                let carry_0 = if i == 0 { 0 } else { data.cout[i * 2 - 1][j] };
                trace[i].set_carry(j, 0, self.to_ranged_field(carry_0, self.carry_range_id));
                trace[i].set_carry(
                    j,
                    1,
                    self.to_ranged_field(data.cout[i * 2][j], self.carry_range_id),
                );
            }
            let q_range_id = if i == BABYJUBJUB_ROWS_BY_OP - 1 {
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
            trace[i].set_A(self.to_ranged_field(data.a[i], self.chunk_range_id) as u16);
            trace[i].set_B(self.to_ranged_field(data.b[i], self.chunk_range_id) as u16);
            trace[i].set_Nx(self.to_ranged_field(data.n[i], self.chunk_range_id) as u16);
            trace[i].set_T(self.to_ranged_field(data.t[i], self.chunk_range_id) as u16);
            trace[i].set_DT(self.to_ranged_field(data.dt[i], self.chunk_range_id) as u16);
            trace[i].set_qa(self.to_ranged_field(data.qa[i], q_range_id) as u32);
            trace[i].set_qb(self.to_ranged_field(data.qb[i], q_range_id) as u32);
            trace[i].set_qn(self.to_ranged_field(data.qn[i], q_range_id) as u32);
            trace[i].set_qt(self.to_ranged_field(data.qt[i], q_range_id) as u32);
            trace[i].set_qdt(self.to_ranged_field(data.qdt[i], q_range_id) as u32);
            trace[i].set_qx(self.to_ranged_field(data.qx[i], q_range_id) as u32);
            trace[i].set_qy(self.to_ranged_field(data.qy[i], q_range_id) as u32);

            for j in 0..BABYJUBJUB_OP_NUM {
                let selected = j == sel_op;
                trace[i].set_sel_op(j, selected);
                if i == 0 {
                    trace[i].set_sel_op_clk0(j, selected);
                } else {
                    trace[i].set_sel_op_clk0(j, false);
                }
            }

            let iclock = match i as u8 {
                Self::FIRST_CLOCK => 1,
                Self::LAST_CLOCK => 2,
                _ => 0,
            };

            // Complete addition: both result coordinates are reduced (< p) and range-checked.
            let x3_lt = data.x3[i] < BABYJUBJUB_PRIME_CHUNKS[i]
                || (i > 0 && data.x3[i] == BABYJUBJUB_PRIME_CHUNKS[i] && prev_x3_lt);
            trace[i].set_x3_lt(x3_lt);
            let row = BabyJubJubLtTableSM::calculate_table_row(
                prev_x3_lt,
                x3_lt,
                data.x3[i] - BABYJUBJUB_PRIME_CHUNKS[i],
                iclock,
            );
            self.std.inc_virtual_row(self.table_id, row as u64, 1);
            prev_x3_lt = x3_lt;

            let y3_lt = data.y3[i] < BABYJUBJUB_PRIME_CHUNKS[i]
                || (i > 0 && data.y3[i] == BABYJUBJUB_PRIME_CHUNKS[i] && prev_y3_lt);
            trace[i].set_y3_lt(y3_lt);
            let row = BabyJubJubLtTableSM::calculate_table_row(
                prev_y3_lt,
                y3_lt,
                data.y3[i] - BABYJUBJUB_PRIME_CHUNKS[i],
                iclock,
            );
            self.std.inc_virtual_row(self.table_id, row as u64, 1);
            prev_y3_lt = y3_lt;
        }
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    pub fn compute_witness<R: BabyJubJubTraceRowOps<F>>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<BabyJubJubInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = BabyJubJubTrace::<R>::new_from_vec(trace_buffer)?;
        let num_rows = trace.num_rows();
        let total_inputs: usize = inputs.iter().map(|x| x.len()).sum();
        let num_rows_needed = total_inputs * BABYJUBJUB_ROWS_BY_OP;

        tracing::debug!(
            "··· Creating BabyJubJub instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );
        let full = num_rows_needed == num_rows;

        timer_start_trace!(BABYJUBJUB_TRACE);

        let mut trace_rows = &mut trace.buffer[..];
        let mut par_traces = Vec::with_capacity(total_inputs);
        // The complete add always sets both lt flags, so the chained previous-row flag is X3|Y3.
        const ALL_LT_FLAGS: u8 = 3;
        let mut previous_lt_flags = 0;
        for (i, inputs) in inputs.iter().enumerate() {
            for (j, _input) in inputs.iter().enumerate() {
                let (head, tail) = trace_rows.split_at_mut(BABYJUBJUB_ROWS_BY_OP);
                par_traces.push((head, i, j, previous_lt_flags));
                previous_lt_flags = ALL_LT_FLAGS;
                trace_rows = tail;
            }
        }
        // If the instance is full, the previous_lt_flag of the first row is the last row's.
        if full {
            par_traces[0].3 = previous_lt_flags;
        }
        let index = par_traces.len();

        par_traces.into_par_iter().for_each(|(trace, i, j, previous_lt_flags)| {
            let input = &inputs[i][j];
            match input {
                BabyJubJubInput::Add(idata) => {
                    self.process_babyjubjub_add(idata, trace, previous_lt_flags)
                }
            }
        });

        let padding_ops = (self.num_available_ops - index) as u64;
        self.std.range_check(self.q_hsc_range_id, 0, 7 * padding_ops);
        self.std.range_check(self.chunk_range_id, 0, 281 * padding_ops);
        self.std.range_check(self.carry_range_id, 0, 224 * padding_ops);

        let padding_row = R::default();

        trace.buffer[num_rows_needed..num_rows].par_iter_mut().for_each(|slot| *slot = padding_row);

        timer_stop_and_log_trace!(BABYJUBJUB_TRACE);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}
