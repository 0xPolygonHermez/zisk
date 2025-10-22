use core::panic;
use std::sync::Arc;

use fields::PrimeField64;
use rayon::prelude::*;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
#[cfg(not(feature = "packed"))]
use zisk_pil::{Sha256fTrace, Sha256fTraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{Sha256fTracePacked, Sha256fTraceRowPacked};

#[cfg(feature = "packed")]
type Sha256fTraceRowType<F> = Sha256fTraceRowPacked<F>;
#[cfg(feature = "packed")]
type Sha256fTraceType<F> = Sha256fTracePacked<F>;

#[cfg(not(feature = "packed"))]
type Sha256fTraceRowType<F> = Sha256fTraceRow<F>;
#[cfg(not(feature = "packed"))]
type Sha256fTraceType<F> = Sha256fTrace<F>;

use super::{sha256f_constants::*, Sha256fInput};

/// The `Sha256fSM` struct encapsulates the logic of the Sha256f State Machine.
pub struct Sha256fSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Number of available sha256fs in the trace.
    pub num_available_sha256fs: usize,

    num_non_usable_rows: usize,

    /// Range checks ID's
    a_range_id: usize,
    e_range_id: usize,
}

impl<F: PrimeField64> Sha256fSM<F> {
    /// Creates a new Sha256f State Machine instance.
    ///
    /// # Returns
    /// A new `Sha256fSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Compute some useful values
        let num_available_sha256fs = Sha256fTraceType::<F>::NUM_ROWS / CLOCKS - 1;
        let num_non_usable_rows = Sha256fTraceType::<F>::NUM_ROWS % CLOCKS;

        let a_range_id = std.get_range_id(0, (1 << 3) - 1, None);
        let e_range_id = std.get_range_id(0, (1 << 3) - 1, None);

        Arc::new(Self { std, num_available_sha256fs, num_non_usable_rows, a_range_id, e_range_id })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Sha256f trace.
    /// * `num_circuits` - The number of circuits to process.
    /// * `input` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    #[inline(always)]
    pub fn process_input(
        &self,
        input: &Sha256fInput,
        trace: &mut [Sha256fTraceRowType<F>],
    ) -> ([u32; 8], [u32; 8]) {
        let mut a_range_checks = [0u32; 8];
        let mut e_range_checks = [0u32; 8];

        let step_main = input.step_main;
        let addr_main = input.addr_main;
        let state_addr = input.state_addr;
        let input_addr = input.input_addr;
        let state = &input.state;
        let input = &input.input;

        // Fill the step_addr
        trace[0].set_step_addr(step_main); // STEP_MAIN
        trace[1].set_step_addr(addr_main as u64); // ADDR_OP
        trace[2].set_step_addr(state_addr as u64); // ADDR_STATE
        trace[3].set_step_addr(input_addr as u64); // ADDR_INPUT
        trace[4].set_step_addr(state_addr as u64); // ADDR_IND_0
        trace[5].set_step_addr(input_addr as u64); // ADDR_IND_1

        // Activate the clk_0 selector
        trace[0].set_in_use_clk_0(true);

        // Activate the in_use selector
        for r in trace.iter_mut().take(18) {
            r.set_in_use(true);
        }

        // Compute the load state stage
        let mut offset = 0;
        let mut prev_state = [0u32; 8];
        for i in 0..CLOCKS_LOAD_STATE {
            let word = state[i];
            let word_high = (word >> 32) as u32;
            let word_low = (word & 0xFFFF_FFFF) as u32;

            // Store the state as u32 for further processing
            prev_state[2 * i] = word_high;
            prev_state[2 * i + 1] = word_low;

            let mut row = if i == 1 || i == 3 { offset + 1 } else { offset + 3 };

            // Locate the state bits in the trace
            let is_a = i < 2;
            for j in 0..32 {
                let bit = ((word_high >> j) & 1) != 0;
                if is_a {
                    trace[row].set_a(j, bit);
                } else {
                    trace[row].set_e(j, bit);
                }
            }
            row -= 1;
            for j in 0..32 {
                let bit = ((word_low >> j) & 1) != 0;
                if is_a {
                    trace[row].set_a(j, bit);
                } else {
                    trace[row].set_e(j, bit);
                }
            }
        }
        offset += CLOCKS_LOAD_STATE;

        // Compute the load input stage
        let mut w = [0u32; 16];
        for i in 0..CLOCKS_LOAD_INPUT {
            let word = input[i / 2];

            // Store the input as u32 for further processing
            w[i] = if i % 2 == 0 { (word >> 32) as u32 } else { (word & 0xFFFF_FFFF) as u32 };

            // Compute the a and e values for the current input
            let [old_a, old_b, old_c, old_d, old_e, old_f, old_g, old_h] = prev_state;
            let (a, e) =
                compute_ae(old_a, old_b, old_c, old_d, old_e, old_f, old_g, old_h, w[i], RC[i]);

            let (a_carry, a) = ((a >> 32) as u8, (a & 0xFFFF_FFFF) as u32);
            let (e_carry, e) = ((e >> 32) as u8, (e & 0xFFFF_FFFF) as u32);

            let row = offset + i;

            // Locate the carry
            trace[row].set_new_a_carry_bits(a_carry);
            trace[row].set_new_e_carry_bits(e_carry);
            a_range_checks[a_carry as usize] += 1;
            e_range_checks[e_carry as usize] += 1;

            // Locate the input bits in the trace
            for j in 0..32 {
                let bit_a = ((a >> j) & 1) != 0;
                let bit_e = ((e >> j) & 1) != 0;
                let bit_w = ((w[i] >> j) & 1) != 0;
                trace[row].set_a(j, bit_a);
                trace[row].set_e(j, bit_e);
                trace[row].set_w(j, bit_w);
            }

            // Update prev_state for the next iteration
            prev_state[7] = old_g;
            prev_state[6] = old_f;
            prev_state[5] = old_e;
            prev_state[4] = e;
            prev_state[3] = old_c;
            prev_state[2] = old_b;
            prev_state[1] = old_a;
            prev_state[0] = a;
        }
        offset += CLOCKS_LOAD_INPUT;

        // Compute the mixing stage
        for i in 0..CLOCKS_MIXING {
            let [old_w2, old_w7, old_w15, old_w16] = [
                w[CLOCKS_LOAD_INPUT - 2],
                w[CLOCKS_LOAD_INPUT - 7],
                w[CLOCKS_LOAD_INPUT - 15],
                w[CLOCKS_LOAD_INPUT - 16],
            ];
            let new_w = compute_w(old_w2, old_w7, old_w15, old_w16);
            let (new_w_carry, new_w) = ((new_w >> 32) as u8, (new_w & 0xFFFF_FFFF) as u32);

            let [old_a, old_b, old_c, old_d, old_e, old_f, old_g, old_h] = prev_state;
            #[rustfmt::skip]
            let (a, e) = compute_ae(old_a, old_b, old_c, old_d, old_e, old_f, old_g, old_h, new_w, RC[CLOCKS_LOAD_INPUT + i]);

            let (a_carry, a) = ((a >> 32) as u8, (a & 0xFFFF_FFFF) as u32);
            let (e_carry, e) = ((e >> 32) as u8, (e & 0xFFFF_FFFF) as u32);

            let row = offset + i;

            // Locate the carry
            trace[row].set_new_a_carry_bits(a_carry);
            trace[row].set_new_e_carry_bits(e_carry);
            trace[row].set_new_w_carry_bits(new_w_carry);
            a_range_checks[a_carry as usize] += 1;
            e_range_checks[e_carry as usize] += 1;

            for j in 0..32 {
                let bit_a = ((a >> j) & 1) != 0;
                let bit_e = ((e >> j) & 1) != 0;
                let bit_w = ((new_w >> j) & 1) != 0;
                trace[row].set_a(j, bit_a);
                trace[row].set_e(j, bit_e);
                trace[row].set_w(j, bit_w);
            }

            // Update prev_state for the next iteration
            prev_state[7] = old_g;
            prev_state[6] = old_f;
            prev_state[5] = old_e;
            prev_state[4] = e;
            prev_state[3] = old_c;
            prev_state[2] = old_b;
            prev_state[1] = old_a;
            prev_state[0] = a;

            // Update the w array for the next iteration
            for j in 0..15 {
                w[j] = w[j + 1];
            }
            w[15] = new_w;
        }
        offset += CLOCKS_MIXING;

        for i in 0..CLOCKS_WRITE_STATE {
            let prev = state[i];
            let prev_high = prev >> 32;
            let prev_low = prev & 0xFFFF_FFFF;

            let curr_high = (prev_state[2 * i]) as u64;
            let curr_low = (prev_state[2 * i + 1]) as u64;

            let new_high = curr_high + prev_high;
            let new_low = curr_low + prev_low;
            let (new_high_carry, new_high) =
                ((new_high >> 32) as u8, (new_high & 0xFFFF_FFFF) as u32);
            let (new_low_carry, new_low) = ((new_low >> 32) as u8, (new_low & 0xFFFF_FFFF) as u32);

            let mut row = if i == 1 || i == 3 { offset + 1 } else { offset + 3 };

            // Locate the state bits in the trace
            let is_a = i < 2;
            if is_a {
                trace[row].set_new_a_carry_bits(new_high_carry);
                a_range_checks[new_high_carry as usize] += 1;
            } else {
                trace[row].set_new_e_carry_bits(new_high_carry);
                e_range_checks[new_high_carry as usize] += 1;
            }

            for j in 0..32 {
                let bit = ((new_high >> j) & 1) != 0;
                if is_a {
                    trace[row].set_a(j, bit);
                } else {
                    trace[row].set_e(j, bit);
                }
            }
            row -= 1;

            if is_a {
                trace[row].set_new_a_carry_bits(new_low_carry);
                a_range_checks[new_low_carry as usize] += 1;
            } else {
                trace[row].set_new_e_carry_bits(new_low_carry);
                e_range_checks[new_low_carry as usize] += 1;
            }

            for j in 0..32 {
                let bit = ((new_low >> j) & 1) != 0;
                if is_a {
                    trace[row].set_a(j, bit);
                } else {
                    trace[row].set_e(j, bit);
                }
            }
        }

        // Perform the zero range checks
        a_range_checks[0] += CLOCKS_LOAD_STATE as u32;
        e_range_checks[0] += CLOCKS_LOAD_STATE as u32;

        return (a_range_checks, e_range_checks);

        #[rustfmt::skip]
        #[allow(clippy::too_many_arguments)]
        fn compute_ae(old_a: u32, old_b: u32, old_c: u32, old_d: u32, old_e: u32, old_f: u32, old_g: u32, old_h: u32, w: u32, k: u32) -> (u64, u64) {
            let s0 = rotate_right(old_a, 2) ^ rotate_right(old_a, 13) ^ rotate_right(old_a, 22);
            let s1 = rotate_right(old_e, 6) ^ rotate_right(old_e, 11) ^ rotate_right(old_e, 25);
            let t1 = (old_h as u64) + (s1 as u64) + (ch(old_e, old_f, old_g) as u64) + (k as u64) + (w as u64);
            let t2 = (s0 as u64) + (maj(old_a, old_b, old_c) as u64);
            let a = (t1 as u64) + (t2 as u64);
            let e = (old_d as u64) + (t1 as u64);
            (a, e)
            // (s0 as u64, s1 as u64)
        }

        fn compute_w(old_w2: u32, old_w7: u32, old_w15: u32, old_w16: u32) -> u64 {
            let s0 = rotate_right(old_w15, 7) ^ rotate_right(old_w15, 18) ^ shift_right(old_w15, 3);
            let s1 = rotate_right(old_w2, 17) ^ rotate_right(old_w2, 19) ^ shift_right(old_w2, 10);
            (s1 as u64) + (old_w7 as u64) + (s0 as u64) + (old_w16 as u64)
        }

        fn rotate_right(x: u32, n: u32) -> u32 {
            (x >> n) | (x << (32 - n))
        }

        fn shift_right(x: u32, n: u32) -> u32 {
            x >> n
        }

        fn maj(x: u32, y: u32, z: u32) -> u32 {
            (x & y) ^ (x & z) ^ (y & z)
        }

        fn ch(x: u32, y: u32, z: u32) -> u32 {
            (x & y) ^ (!x & z)
        }
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `sctx` - The setup context containing the setup data.
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness(
        &self,
        inputs: &[Vec<Sha256fInput>],
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut sha256f_trace = Sha256fTraceType::new_from_vec_zeroes(trace_buffer);
        let num_rows = sha256f_trace.num_rows();
        let num_available_sha256fs = self.num_available_sha256fs;

        let mut a_range_checks = vec![0; 1 << 3];
        let mut e_range_checks = vec![0; 1 << 3];

        // Check that we can fit all the sha256fs in the trace
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        let num_rows_filled = num_inputs * CLOCKS;
        let num_rows_needed = if num_inputs < num_available_sha256fs {
            num_inputs * CLOCKS
        } else if num_inputs == num_available_sha256fs {
            num_rows
        } else {
            panic!(
                "Exceeded available Sha256fs inputs: requested {}, but only {} are available.",
                num_inputs, self.num_available_sha256fs
            );
        };

        tracing::info!(
            "··· Creating Sha256f instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(SHA256F_TRACE);
        let mut trace_rows = sha256f_trace.buffer.as_mut_slice();
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

        // Fill the trace
        let input_range_checks: Vec<([u32; 8], [u32; 8])> = par_traces
            .into_par_iter()
            .enumerate()
            .map(|(index, trace)| {
                let input_index = inputs_indexes[index];
                let input = &inputs[input_index.0][input_index.1];
                self.process_input(input, trace)
            })
            .collect();

        for (a_inp_range_checks, e_inp_range_checks) in input_range_checks {
            for i in 0..8 {
                a_range_checks[i] += a_inp_range_checks[i];
                e_range_checks[i] += e_inp_range_checks[i];
            }
        }

        timer_stop_and_log_trace!(SHA256F_TRACE);

        timer_start_trace!(SHA256F_PADDING);
        // Set a = e = w = 0 for the state and input rows
        let zero_row = Sha256fTraceRowType::<F>::default();

        // precompute compute_ae() with initial a = e = 0 (PC_A and PC_E)
        // compute_w() with w = 0 is equal to 0, nothing to do
        let mut mid_rows = [Sha256fTraceRowType::<F>::default(); 64];
        for i in 0..64 {
            let a = PC_A[i];
            let e = PC_E[i];
            let (a_carry, a) = ((a >> 32) as u8, (a & 0xFFFF_FFFF) as u32);
            let (e_carry, e) = ((e >> 32) as u8, (e & 0xFFFF_FFFF) as u32);
            mid_rows[i].set_new_a_carry_bits(a_carry);
            mid_rows[i].set_new_e_carry_bits(e_carry);
            for j in 0..32 {
                let bit_a = ((a >> j) & 1) != 0;
                let bit_e = ((e >> j) & 1) != 0;
                mid_rows[i].set_a(j, bit_a);
                mid_rows[i].set_e(j, bit_e);
            }

            a_range_checks[a_carry as usize] += (num_available_sha256fs - num_inputs) as u32;
            e_range_checks[e_carry as usize] += (num_available_sha256fs - num_inputs) as u32;
        }

        // At the end, we should have that a === 4'and e === 4'e
        let mut final_rows = [Sha256fTraceRowType::<F>::default(); 4];
        for i in 0..4 {
            let a = (PC_A[60 + i] & 0xFFFF_FFFF) as u32;
            let e = (PC_E[60 + i] & 0xFFFF_FFFF) as u32;
            for j in 0..32 {
                let bit_a = ((a >> j) & 1) != 0;
                let bit_e = ((e >> j) & 1) != 0;
                final_rows[i].set_a(j, bit_a);
                final_rows[i].set_e(j, bit_e);
            }
        }

        const CLOCKS_OP: usize = CLOCKS_LOAD_STATE + CLOCKS_LOAD_INPUT + CLOCKS_MIXING;
        // The last (CLOCKS + NUM_NON_USABLE_ROWS) have CLK_0 desactivated, so
        // a trace full of zeroes passes the constraints
        sha256f_trace.buffer[num_rows_filled..(num_rows - self.num_non_usable_rows - CLOCKS)]
            .par_iter_mut()
            .enumerate()
            .for_each(|(elem, row)| {
                let row_r = elem % CLOCKS;
                if row_r < CLOCKS_LOAD_STATE {
                    *row = zero_row;
                } else if row_r < CLOCKS_OP {
                    *row = mid_rows[row_r - CLOCKS_LOAD_STATE];
                } else {
                    *row = final_rows[row_r - CLOCKS_OP];
                }
            });

        // Perform the zero range checks
        let count_zeros = (num_available_sha256fs - num_inputs)
            * (CLOCKS_LOAD_STATE + CLOCKS_WRITE_STATE)
            + CLOCKS
            + self.num_non_usable_rows;
        a_range_checks[0] += count_zeros as u32;
        e_range_checks[0] += count_zeros as u32;

        self.std.range_checks(self.a_range_id, a_range_checks);
        self.std.range_checks(self.e_range_id, e_range_checks);

        timer_stop_and_log_trace!(SHA256F_PADDING);

        AirInstance::new_from_trace(FromTrace::new(&mut sha256f_trace))
    }
}
