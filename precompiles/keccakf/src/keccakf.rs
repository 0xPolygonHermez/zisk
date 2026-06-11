use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;

use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

use precompiles_helpers::{
    keccak_f_round, keccakf_bit_pos, keccakf_state_flatten, keccakf_state_from_linear,
};
use zisk_common::OperationKeccakData;
use zisk_pil::{KeccakfTrace, KeccakfTraceRowOps};

use super::{keccakf_constants::*, KeccakfTableSM};

use rayon::prelude::*;

/// Per-operation input record assembled from the bus payload.
#[derive(Debug)]
pub struct KeccakfInput {
    pub step_main: u64,
    pub addr_main: u32,
    pub state: [u64; 25],
}

impl KeccakfInput {
    pub fn from(values: &OperationKeccakData<u64>) -> Self {
        Self {
            step_main: values[4],
            addr_main: values[3] as u32,
            state: values[5..30].try_into().unwrap(),
        }
    }
}

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
        let num_non_usable_rows = KeccakfTrace::<()>::NUM_ROWS % CLOCKS;
        let num_available_keccakfs = if num_non_usable_rows == 0 {
            KeccakfTrace::<()>::NUM_ROWS / CLOCKS
        } else {
            // Subtract 1 because we can't fit a complete cycle in the remaining rows
            (KeccakfTrace::<()>::NUM_ROWS - num_non_usable_rows) / CLOCKS - 1
        };

        // Get the table ID
        let table_id = std
            .get_virtual_table_id(KeccakfTableSM::TABLE_ID)
            .expect("Failed to get Keccakf table ID");

        Arc::new(Self { num_available_keccakfs, std, table_id })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Keccakf trace.
    /// * `input` - The operation data to process.
    #[inline(always)]
    #[allow(clippy::needless_range_loop)]
    fn process_trace<R: KeccakfTraceRowOps<F>>(
        &self,
        trace: &mut [R],
        input: &[u64; 25],
        addr: u32,
        step: u64,
    ) {
        // Fill step and addr
        trace[0].set_step_addr(step);
        trace[1].set_step_addr(addr as u64);

        // Fill in_use
        for i in 0..CLOCKS {
            trace[i].set_in_use(true);
        }

        // Convert input state to 5x5x64 representation
        let mut state = keccakf_state_from_linear(input);

        // Row 0: fill the input state
        let state_flat = keccakf_state_flatten(&state);

        // Allocate buffers once and reuse across all rounds - better performance
        let mut accs = [0u32; NUM_CHUNKS];
        let mut state_bits = [false; 1600]; // 5 * 5 * 64 = 1600 bits

        for (i, &val) in state_flat.iter().enumerate() {
            state_bits[i] = (val & 1) != 0;
        }
        trace[0].set_all_state(&state_bits);

        // Rows 1..CLOCKS: apply each round
        for r in 0..ROUNDS {
            // Apply round function to the state
            keccak_f_round(&mut state, r);

            // Flatten unreduced state for accumulator computation
            let state_flat = keccakf_state_flatten(&state);

            // Compute accumulators (reusing accs buffer)
            for i in 0..NUM_CHUNKS {
                let offset = i * TABLE_MAX_CHUNKS;
                let num_bits = std::cmp::min(TABLE_MAX_CHUNKS, WIDTH - offset);

                let mut acc = 0u32;
                for j in 0..num_bits {
                    acc += (state_flat[offset + j] as u32) * POWS_BASE[j];
                }
                accs[i] = acc;
            }
            trace[r].set_all_chunk_acc(&accs);

            // Reduce the state modulo 2 and collect all state bits (reusing state_bits buffer)
            for x in 0..5 {
                for y in 0..5 {
                    for z in 0..64 {
                        // Reduce the state modulo 2
                        state[x][y][z] %= 2;

                        // Collect the bit
                        let bit_pos = keccakf_bit_pos(x, y, z);
                        state_bits[bit_pos] = state[x][y][z] == 1;
                    }
                }
            }

            // Fill the trace for the next round all at once
            trace[r + 1].set_all_state(&state_bits);
        }
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness<R: KeccakfTraceRowOps<F>>(
        &self,
        inputs: &[Vec<KeccakfInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = KeccakfTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        // Check that we can fit all the keccakfs in the trace
        let num_available_keccakfs = self.num_available_keccakfs;
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        let num_rows_needed = if num_inputs < num_available_keccakfs {
            num_inputs * CLOCKS
        } else if num_inputs == num_available_keccakfs {
            num_rows
        } else {
            panic!(
                "Exceeded available Keccakfs inputs: requested {}, but only {} are available.",
                num_inputs, num_available_keccakfs
            );
        };

        tracing::debug!(
            "··· Creating Keccakf instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(KECCAKF_TRACE);

        // 1] Fill the trace with the provided inputs
        let mut trace_rows = &mut trace.buffer[..];
        let mut par_traces = Vec::new();
        let mut inputs_indexes = Vec::new();
        for (i, inputs) in inputs.iter().enumerate() {
            for (j, _) in inputs.iter().enumerate() {
                let (head, tail) = trace_rows.split_at_mut(CLOCKS);
                par_traces.push(head);
                inputs_indexes.push((i, j));
                trace_rows = tail;
            }
        }

        par_traces.par_iter_mut().enumerate().for_each(|(index, trace)| {
            let input_index = inputs_indexes[index];
            let input = &inputs[input_index.0][input_index.1];
            self.process_trace::<R>(trace, &input.state, input.addr_main, input.step_main);
        });

        // 2] Update lookup table
        let mut table = vec![0u32; TABLE_SIZE as usize];
        for keccak_idx in 0..num_inputs {
            let base_row = keccak_idx * CLOCKS;
            // Each keccak has 24 rounds of accumulators (stored in rows 0..23 of each keccak block)
            for round in 0..ROUNDS {
                let chunk_accs = trace.buffer[base_row + round].get_all_chunk_acc();
                for acc in chunk_accs.iter() {
                    let table_row = KeccakfTableSM::calculate_table_row(*acc);
                    table[table_row as usize] += 1;
                }
            }
        }
        table.into_par_iter().enumerate().for_each(|(row, value)| {
            if value > 0 {
                self.std.inc_virtual_row(self.table_id, row as u32, value);
            }
        });
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}
