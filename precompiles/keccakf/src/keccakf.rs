use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;

use proofman_common::{AirInstance, FromTrace};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

#[cfg(not(feature = "packed"))]
use zisk_pil::{KeccakfTrace, KeccakfTraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{KeccakfTracePacked, KeccakfTraceRowPacked};

#[cfg(feature = "packed")]
type KeccakfTraceRowType<F> = KeccakfTraceRowPacked<F>;
#[cfg(feature = "packed")]
type KeccakfTraceType<F> = KeccakfTracePacked<F>;

#[cfg(not(feature = "packed"))]
type KeccakfTraceRowType<F> = KeccakfTraceRow<F>;
#[cfg(not(feature = "packed"))]
type KeccakfTraceType<F> = KeccakfTrace<F>;

use precompiles_helpers::keccak_f_round_states;

use crate::KeccakfInput;

use super::{keccakf_constants::*, KeccakfTableSM};

use rayon::prelude::*;

/// The `KeccakfSM` struct encapsulates the logic of the Keccakf State Machine.
pub struct KeccakfSM<F: PrimeField64> {
    /// Number of available keccakfs in the trace.
    pub num_available_keccakfs: usize,

    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,

    /// The table ID for the Keccakf Table State Machine
    table_id: usize,
}

impl<F: PrimeField64> KeccakfSM<F> {
    /// Creates a new Keccakf State Machine instance.
    ///
    /// # Arguments
    /// * `keccakf_table_sm` - An `Arc`-wrapped reference to the Keccakf Table State Machine.
    ///
    /// # Returns
    /// A new `KeccakfSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Compute some useful values
        let num_non_usable_rows = KeccakfTrace::<F>::NUM_ROWS % ROWS_BY_KECCAKF;
        let num_available_keccakfs = if num_non_usable_rows == 0 {
            KeccakfTrace::<F>::NUM_ROWS / ROWS_BY_KECCAKF
        } else {
            // The -1 is because ROWS_BY_KECCAKF is not a divisor of N
            (KeccakfTrace::<F>::NUM_ROWS - num_non_usable_rows) / ROWS_BY_KECCAKF - 1
        };

        // Get the table ID
        let table_id = std.get_virtual_table_id(KeccakfTableSM::TABLE_ID);

        Arc::new(Self { num_available_keccakfs, std, table_id })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Keccakf trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    pub fn process_trace(&self, trace: &mut [KeccakfTraceRow<F>], input: &KeccakfInput) {
        let state = &input.state;

        // Fill the input state
        for i in 0..WORDS {
            let word = state[i];
            let offset = i * 64;
            for j in 0..64 {
                let bit = (word >> j) & 1;
                trace[0].set_state(offset + j, bit == 1);
            }
        }

        // Fill the rest of states
        let round_states = keccak_f_round_states(state.clone());
        for (i, round_state) in round_states.enumerate() {
            for j in 0..WORDS {
                let word = round_state[j];
                let offset = j * 64;
                for k in 0..64 {
                    let bit = (word >> k) & 1;
                    trace[1 + i].set_state(offset + k, bit == 1);
                }
            }
        }

        // Fill step and addr
        let step_main = input.step_main;
        let addr_main = input.addr_main;
        trace[0].set_step_addr(step_main);
        trace[1].set_step_addr(addr_main as u64);

        // Fill in_use_clk_0
        trace[0].set_in_use_clk_0(true);

        // Fill in_use
        for i in 0..ROWS_BY_KECCAKF {
            trace[i].set_in_use(true);
        }

        // KeccakfTableSM::calculate_table_row(&gate_op_val, a_val, b_val, c_val);
        // self.std.inc_virtual_row(self.table_id, table_row as u64, 1);
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
        inputs: &[Vec<KeccakfInput>],
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        timer_start_trace!(KECCAKF_TRACE);
        let mut trace = KeccakfTrace::new_from_vec_zeroes(trace_buffer);
        let num_rows = trace.num_rows();

        // Check that we can fit all the keccakfs in the trace
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        let num_rows_needed = num_inputs * ROWS_BY_KECCAKF;

        // Sanity checks
        debug_assert!(
            num_inputs <= self.num_available_keccakfs,
            "Exceeded available Keccakfs inputs: requested {}, but only {} are available.",
            num_inputs,
            self.num_available_keccakfs
        );
        debug_assert!(num_rows_needed <= num_rows);

        // TODO: Add remaining rows when instance is fully filled
        tracing::info!(
            "··· Creating Keccakf instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        // Fill the trace
        let mut trace_rows = &mut trace.buffer[..];
        let mut par_traces = Vec::new();
        let mut inputs_indexes = Vec::new();
        for (i, inputs) in inputs.iter().enumerate() {
            for (j, _) in inputs.iter().enumerate() {
                let (head, tail) = trace_rows.split_at_mut(ROWS_BY_KECCAKF);
                par_traces.push(head);
                inputs_indexes.push((i, j));
                trace_rows = tail;
            }
        }
        // let index = par_traces.len();

        par_traces.into_par_iter().enumerate().for_each(|(index, trace)| {
            let input_index = inputs_indexes[index];
            let input = &inputs[input_index.0][input_index.1];
            self.process_trace(trace, input);
        });
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        // TODO:
        timer_start_trace!(KECCAKF_PADDING);

        // let padding_ops = (self.num_available_keccakfs - index) as u64;
        let padding_row: KeccakfTraceRow<F> = Default::default();

        trace.buffer[num_rows_needed..num_rows].par_iter_mut().for_each(|slot| *slot = padding_row);

        timer_stop_and_log_trace!(KECCAKF_PADDING);

        // // A row with all zeros satisfies the constraints (since XOR(0,0,0) = 0)
        // let padding_row: KeccakfTraceRow<F> = Default::default();
        // for i in (num_rows_constants + self.circuit_size * self.num_available_circuits)..num_rows {
        //     let gate_op = self.keccakf_fixed[i].GATE_OP.as_canonical_u64();
        //     // Sanity check
        //     debug_assert_eq!(
        //         gate_op,
        //         KeccakfTableGateOp::Xor as u64,
        //         "Invalid padding dummy gate operation"
        //     );

        //     let table_row =
        //         KeccakfTableSM::calculate_table_row(&KeccakfTableGateOp::Xor, zeros, zeros, zeros);
        //     self.std.inc_virtual_row(self.table_id, table_row as u64, CHUNKS_KECCAKF as u64);

        //     keccakf_trace[i] = padding_row;
        // }
        // timer_stop_and_log_trace!(KECCAKF_PADDING);

        AirInstance::new_from_trace(FromTrace::new(&mut trace))
    }
}
