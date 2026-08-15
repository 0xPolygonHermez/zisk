use std::sync::Arc;

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;

use proofman_common::{AirInstance, FromTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

use zisk_common::OperationKeccakData;
use zisk_pil::{KeccakfTrace, KeccakfTraceRowOps};
use zisk_precomp_helpers::{
    keccak_f_chi_iota, keccak_f_theta_rho_pi, keccakf_bit_pos, keccakf_state_from_linear,
    KECCAK_F_RC_BITS,
};

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
    /// * `std` - An `Arc`-wrapped reference to the PIL2 standard library.
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

    /// Processes one operation: fills its CLOCKS-row block of the trace and
    /// accumulates the χ-row S-box lookups into the table histogram.
    ///
    /// The trace holds one bit per state cell, with each row storing the true
    /// (post-ι) round state. Per round, the AIR looks up one χ-row S-box per
    /// (y, z) pair, whose input packs the five θρπ-outputs (values <= 11) in
    /// base 12 plus the ι bit; the table row indexes mirror that packing.
    ///
    /// # Arguments
    /// * `trace` - The CLOCKS-row block of the trace assigned to this operation.
    /// * `input` - The input state of the operation.
    /// * `addr` - The main address of the operation.
    /// * `step` - The main step of the operation.
    /// * `table` - The multiplicity histogram of the Keccakf table.
    #[inline(always)]
    #[allow(clippy::needless_range_loop)]
    fn process_trace<R: KeccakfTraceRowOps<F>>(
        &self,
        trace: &mut [R],
        input: &[u64; 25],
        addr: u32,
        step: u64,
        table: &mut [u32],
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
        let mut state_bits = [false; WIDTH];
        for x in 0..5 {
            for y in 0..5 {
                for z in 0..64 {
                    state_bits[keccakf_bit_pos(x, y, z)] = state[x][y][z] == 1;
                }
            }
        }
        trace[0].set_all_state(&state_bits);

        // Rows 1..CLOCKS: apply each round
        let mut t = [0u8; 5];
        for r in 0..ROUNDS {
            // θ + ρπ: the state now holds the χ-row inputs B (values <= 11)
            keccak_f_theta_rho_pi(&mut state);

            // One S-box lookup per χ-row (y, z); only y = 0 rows carry the ι bit
            for y in 0..5 {
                for z in 0..64 {
                    for x in 0..5 {
                        t[x] = state[x][y][z];
                    }
                    let rc = y == 0 && KECCAK_F_RC_BITS[r][z];
                    let row = KeccakfTableSM::calculate_table_row(&t, rc);
                    table[row as usize] += 1;
                }
            }

            // χ + ι, then reduce modulo 2 to get the true round output
            keccak_f_chi_iota(&mut state, r);
            for x in 0..5 {
                for y in 0..5 {
                    for z in 0..64 {
                        state[x][y][z] %= 2;
                        state_bits[keccakf_bit_pos(x, y, z)] = state[x][y][z] == 1;
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
        _sctx: &SetupCtx<F>,
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

        // Pair each input with its CLOCKS-row block of the trace
        let mut trace_rows = &mut trace.buffer[..];
        let mut par_traces = Vec::with_capacity(num_inputs);
        let mut flat_inputs = Vec::with_capacity(num_inputs);
        for inputs in inputs.iter() {
            for input in inputs.iter() {
                let (head, tail) = trace_rows.split_at_mut(CLOCKS);
                par_traces.push(head);
                flat_inputs.push(input);
                trace_rows = tail;
            }
        }

        // Fill the trace and accumulate the table histogram in parallel
        let table: Vec<u32> = par_traces
            .into_par_iter()
            .zip(flat_inputs.into_par_iter())
            .fold(
                || vec![0u32; TABLE_SIZE as usize],
                |mut table, (trace, input)| {
                    self.process_trace::<R>(
                        trace,
                        &input.state,
                        input.addr_main,
                        input.step_main,
                        &mut table,
                    );
                    table
                },
            )
            .reduce(
                || vec![0u32; TABLE_SIZE as usize],
                |mut acc, partial| {
                    acc.iter_mut().zip(partial.iter()).for_each(|(a, p)| *a += p);
                    acc
                },
            );

        // Update the lookup table multiplicities
        table.into_par_iter().enumerate().for_each(|(row, value)| {
            if value > 0 {
                self.std.inc_virtual_row(self.table_id, row as u32, value);
            }
        });
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}
