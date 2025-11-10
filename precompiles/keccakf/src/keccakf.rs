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

#[cfg(not(feature = "packed"))]
type KeccakfTraceRowType<F> = KeccakfTraceRow<F>;

use precompiles_helpers::{keccak_f_rounds, state_from_linear};

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
    const BASE: u32 = MAX_VALUE + 1;

    const NUM_REM: usize = 1600 % GROUP_BY;
    const NUM_REDUCED: usize = (1600 - Self::NUM_REM) / GROUP_BY;

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
        let num_available_keccakfs =
            (KeccakfTrace::<F>::NUM_ROWS - num_non_usable_rows) / ROWS_BY_KECCAKF;

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
    fn process_trace(&self, trace: &mut [KeccakfTraceRow<F>], input: &KeccakfInput) {
        let state = &input.state;

        // Convert input state to 5x5x64 representation
        let initial_state = state_from_linear(state);

        // Fill the input state
        for x in 0..5 {
            for y in 0..5 {
                for z in 0..64 {
                    trace[0].set_state(x, y, z, (initial_state[x][y][z] % 2) == 1);
                }
            }
        }

        // Fill the rest of states
        let round_states = keccak_f_rounds(initial_state);
        for (state, r) in round_states {
            // Fill keccakf_state
            for x in 0..5 {
                for y in 0..5 {
                    for z in 0..64 {
                        trace[1 + r].set_state(x, y, z, (state[x][y][z] % 2) == 1);
                    }
                }
            }

            // Fill keccakf_reduced
            for i in 0..Self::NUM_REDUCED {
                let offset = i * GROUP_BY;
                let mut acc = 0u32;
                for j in 0..GROUP_BY {
                    let idx = offset + j;
                    let (x, y, z) = Self::idx_pos(idx);
                    let value = state[x][y][z] as u32;
                    acc += value * Self::BASE.pow(j as u32);
                }
                trace[r].set_keccakf_reduced(i, acc);

                let table_row = KeccakfTableSM::calculate_table_row(acc);
                self.std.inc_virtual_row(self.table_id, table_row as u64, 1);
            }

            // Fill keccakf_rem
            let offset = Self::NUM_REDUCED * GROUP_BY;
            let mut acc = 0u8;
            for j in 0..Self::NUM_REM {
                let idx = offset + j;
                let (x, y, z) = Self::idx_pos(idx);
                let bit_value = state[x][y][z] as u8;
                acc += bit_value * (Self::BASE.pow(j as u32) as u8);
            }
            trace[r].set_keccakf_rem(acc);

            let table_row = KeccakfTableSM::calculate_table_row(acc as u32);
            self.std.inc_virtual_row(self.table_id, table_row as u64, 1);
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
    }

    fn idx_pos(idx: usize) -> (usize, usize, usize) {
        debug_assert!(idx < 1600);

        let x = (idx / 64) % 5;
        let y = (idx / 320) % 5;
        let z = idx % 64;
        (x, y, z)
    }

    #[inline(always)]
    fn process_padding(&self, trace: &mut [KeccakfTraceRow<F>]) {
        // Fill the rest of states
        let initial_state = [[[0u64; 64]; 5]; 5];
        let round_states = keccak_f_rounds(initial_state);
        const NUM_REM: usize = 1600 % GROUP_BY;
        const NUM_REDUCED: usize = (1600 - NUM_REM) / GROUP_BY;
        for (state, r) in round_states {
            // Fill keccakf_state
            for x in 0..5 {
                for y in 0..5 {
                    for z in 0..64 {
                        trace[1 + r].set_state(x, y, z, (state[x][y][z] % 2) == 1);
                    }
                }
            }

            // Fill keccakf_reduced
            for i in 0..NUM_REDUCED {
                let offset = i * GROUP_BY;
                let mut acc = 0u32;
                for j in 0..GROUP_BY {
                    let idx = offset + j;
                    let (x, y, z) = Self::idx_pos(idx);
                    let value = state[x][y][z] as u32;
                    acc += value * Self::BASE.pow(j as u32);
                }
                trace[r].set_keccakf_reduced(i, acc);

                let table_row = KeccakfTableSM::calculate_table_row(acc);
                self.std.inc_virtual_row(self.table_id, table_row as u64, 1);
            }

            // Fill keccakf_rem
            let offset = NUM_REDUCED * GROUP_BY;
            let mut acc = 0u8;
            for j in 0..NUM_REM {
                let idx = offset + j;
                let (x, y, z) = Self::idx_pos(idx);
                let bit_value = state[x][y][z] as u8;
                acc += bit_value * (Self::BASE.pow(j as u32) as u8);
            }
            trace[r].set_keccakf_rem(acc);

            let table_row = KeccakfTableSM::calculate_table_row(acc as u32);
            self.std.inc_virtual_row(self.table_id, table_row as u64, 1);
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

        par_traces.into_par_iter().enumerate().for_each(|(index, trace)| {
            let input_index = inputs_indexes[index];
            let input = &inputs[input_index.0][input_index.1];
            self.process_trace(trace, input);
        });
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        timer_start_trace!(KECCAKF_PADDING);

        // Set the padding rows with Keccakf(0)
        let padding_rows_start = num_rows_needed;
        let padding_rows_end =
            padding_rows_start + ((self.num_available_keccakfs - num_inputs) * ROWS_BY_KECCAKF);

        // Split the padding trace into padding chunks
        let padding_trace = &mut trace.buffer[padding_rows_start..padding_rows_end];
        let padding_chunks: Vec<_> = padding_trace.chunks_mut(ROWS_BY_KECCAKF).collect();

        // Process padding in parallel
        padding_chunks.into_par_iter().for_each(|trace_chunk| {
            self.process_padding(trace_chunk);
        });

        // Set the remaining rows with 0's
        let remaining_rows = KeccakfTrace::<F>::NUM_ROWS - padding_rows_end;
        let zeroes_rows = self.num_available_keccakfs + remaining_rows;
        let multiplicity = zeroes_rows * (Self::NUM_REDUCED + Self::NUM_REM);
        let table_row = KeccakfTableSM::calculate_table_row(0);
        self.std.inc_virtual_row(self.table_id, table_row as u64, multiplicity as u64);

        timer_stop_and_log_trace!(KECCAKF_PADDING);

        AirInstance::new_from_trace(FromTrace::new(&mut trace))
    }
}
