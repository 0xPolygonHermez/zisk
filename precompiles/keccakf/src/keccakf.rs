use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;

use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

#[cfg(not(feature = "packed"))]
use zisk_pil::{KeccakfTrace, KeccakfTraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{KeccakfTracePacked, KeccakfTraceRowPacked};

#[cfg(feature = "packed")]
type KeccakfTraceType<F> = KeccakfTracePacked<F>;

#[cfg(not(feature = "packed"))]
type KeccakfTraceType<F> = KeccakfTrace<F>;

use precompiles_helpers::{keccak_f_rounds, keccakf_state_from_linear};

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
    const NUM_REM: usize = 1600 % TABLE_CHUNK_SIZE;
    const NUM_REDUCED: usize = (1600 - Self::NUM_REM) / TABLE_CHUNK_SIZE;

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
            // Subtract 1 because we can't fit a complete cycle in the remaining rows
            (KeccakfTrace::<F>::NUM_ROWS - num_non_usable_rows) / ROWS_BY_KECCAKF - 1
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
    fn process_trace(
        &self,
        trace: &mut [KeccakfTraceRow<F>],
        initial_state: &[u64; 25],
        addr: Option<u32>,
        step: Option<u64>,
    ) {
        let lookup_active = addr.is_some() && step.is_some();

        // Fill the states
        // Convert input state to 5x5x64 representation
        let initial_state = keccakf_state_from_linear(initial_state);
        let round_states = keccak_f_rounds(initial_state);
        for (state, r) in round_states {
            // Fill keccakf_state
            for x in 0..5 {
                for y in 0..5 {
                    for z in 0..64 {
                        trace[r].set_state(x, y, z, (state[x][y][z] % 2) == 1);
                    }
                }
            }

            // Fill keccakf_reduced
            for i in 0..Self::NUM_REDUCED {
                let offset = i * TABLE_CHUNK_SIZE;
                let mut acc = 0u32;
                for j in 0..TABLE_CHUNK_SIZE {
                    let idx = offset + j;
                    let (x, y, z) = Self::idx_pos(idx);
                    let value = state[x][y][z] as u32;
                    acc += value * BASE.pow(j as u32);
                }
                if r > 0 {
                    trace[r - 1].set_chunk_acc(i, acc);

                    if lookup_active {
                        let table_row = KeccakfTableSM::calculate_table_row(acc);
                        self.std.inc_virtual_row(self.table_id, table_row as u64, 1);
                    }
                }
            }

            // Fill keccakf_rem
            let offset = Self::NUM_REDUCED * TABLE_CHUNK_SIZE;
            let mut acc = 0u8;
            for j in 0..Self::NUM_REM {
                let idx = offset + j;
                let (x, y, z) = Self::idx_pos(idx);
                let bit_value = state[x][y][z] as u8;
                acc += bit_value * (BASE.pow(j as u32) as u8);
            }
            if r > 0 {
                trace[r - 1].set_rem_acc(acc);

                if lookup_active {
                    let table_row = KeccakfTableSM::calculate_table_row(acc as u32);
                    self.std.inc_virtual_row(self.table_id, table_row as u64, 1);
                }
            }
        }

        if !lookup_active {
            return;
        }

        // Fill step and addr
        trace[0].set_step_addr(step.unwrap());
        trace[1].set_step_addr(addr.unwrap() as u64);

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
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = KeccakfTraceType::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        // Check that we can fit all the keccakfs in the trace
        let num_available_keccakfs = self.num_available_keccakfs;
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        let num_rows_needed = if num_inputs < num_available_keccakfs {
            num_inputs * ROWS_BY_KECCAKF
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
                let (head, tail) = trace_rows.split_at_mut(ROWS_BY_KECCAKF);
                par_traces.push(head);
                inputs_indexes.push((i, j));
                trace_rows = tail;
            }
        }

        par_traces.into_par_iter().enumerate().for_each(|(index, trace)| {
            let input_index = inputs_indexes[index];
            let input = &inputs[input_index.0][input_index.1];
            self.process_trace(trace, &input.state, Some(input.addr_main), Some(input.step_main));
        });

        timer_stop_and_log_trace!(KECCAKF_TRACE);

        timer_start_trace!(KECCAKF_PADDING);

        // 2] Fill the padding rows with Keccakf(0)
        let padding_rows_start = num_rows_needed;
        let padding_rows_end =
            padding_rows_start + ((num_available_keccakfs - num_inputs) * ROWS_BY_KECCAKF);

        // Split the padding trace into padding chunks
        let padding_trace = &mut trace.buffer[padding_rows_start..padding_rows_end];
        let padding_chunks: Vec<_> = padding_trace.chunks_mut(ROWS_BY_KECCAKF).collect();

        // Process padding in parallel
        padding_chunks.into_par_iter().for_each(|trace_chunk| {
            self.process_trace(trace_chunk, &[0u64; 25], None, None);
        });

        // 3] The non-usable rows should be zeroes, which are already set at initialization

        timer_stop_and_log_trace!(KECCAKF_PADDING);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}
