use core::panic;
use std::{f32::consts::E, sync::Arc};

use fields::PrimeField64;

use proofman_common::{AirInstance, FromTrace, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_pil::{Sha256fDirectTrace, Sha256fDirectTraceRow};

use crate::Sha256fInput;

use super::sha256f_constants::*;

use rayon::prelude::*;

/// The `Sha256fSM` struct encapsulates the logic of the Sha256f State Machine.
pub struct Sha256fSM {
    /// Number of available sha256fs in the trace.
    pub num_available_sha256fs: usize,

    /// The number of rows that cannot be used
    pub num_unusable_rows: usize,
}

impl Sha256fSM {
    /// Creates a new Sha256f State Machine instance.
    ///
    /// # Returns
    /// A new `Sha256fSM` instance.
    pub fn new() -> Arc<Self> {
        // Compute some useful values
        let num_available_sha256fs = Sha256fDirectTrace::<usize>::NUM_ROWS / CLOCKS;
        let num_unusable_rows = Sha256fDirectTrace::<usize>::NUM_ROWS % CLOCKS;

        Arc::new(Self { num_available_sha256fs, num_unusable_rows })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Sha256f trace.
    /// * `num_circuits` - The number of circuits to process.
    /// * `input` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    #[inline(always)]
    pub fn process_sha256f<F: PrimeField64>(
        &self,
        input: &Sha256fInput,
        trace: &mut Sha256fDirectTrace<F>,
        row_offset: usize,
    ) {
        let step_received = input.step_main;
        let addr_received = input.addr_main;
        let state_addr_received = input.state_addr;
        let input_addr_received = input.input_addr;
        let state_received = &input.state;
        let input_received = &input.input;
        println!("Processing Sha256f input: state={:x?}, input={:x?}",state_received, input_received);

        let mut prev_state = [0u32; 8];
        for (i, word) in state_received.iter().enumerate() {
            // Store the state as u32 for further processing
            prev_state[2 * i] = (word >> 32) as u32;
            prev_state[2 * i + 1] = (word & 0xFFFF_FFFF) as u32;

            // Locate the state bits in the trace
            let mut pos = if i == 1 || i == 3 { row_offset + 1 } else { row_offset + 3 };

            let is_a = i < 2;
            for j in 0..64 {
                let bit = ((word >> j) & 1) as u8;
                if j == 32 {
                    pos -= 1;
                }

                if is_a {
                    trace[pos].a[j % 32] = F::from_u8(bit);
                } else {
                    trace[pos].e[j % 32] = F::from_u8(bit);
                }
            }
        }

        let mut w = [0u32; 16];
        for (i, word) in input_received.iter().enumerate() {
            // Store the input as u32 for further processing
            w[2 * i] = (word >> 32) as u32;
            w[2 * i + 1] = (word & 0xFFFF_FFFF) as u32;

            // Locate the input bits in the trace
            let mut pos = row_offset + 4 + 2 * i;

            for j in 0..64 {
                let bit = ((word >> j) & 1) as u8;
                if j == 32 {
                    pos += 1;
                }

                trace[pos].w[j % 32] = F::from_u8(bit);
            }
        }

        // TODO: Mix the previous loop with the following one

        // Compute the mixing & load input part
        let mut offset = row_offset + CLOCKS_LOAD_STATE;
        for i in 0..CLOCKS_LOAD_INPUT {
            let [old_a, old_b, old_c, old_d, old_e, old_f, old_g, old_h] = prev_state;
            let (a, e) =
                compute_ae(old_a, old_b, old_c, old_d, old_e, old_f, old_g, old_h, w[i], RC[i]);

            let pos = offset + i;
            for j in 0..32 {
                let bit_a = ((a >> j) & 1) as u8;
                let bit_e = ((e >> j) & 1) as u8;
                trace[pos].a[j] = F::from_u8(bit_a);
                trace[pos].e[j] = F::from_u8(bit_e);
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

        // // Compute the mixing part
        // offset += CLOCKS_LOAD_INPUT;
        // for i in 0..CLOCKS_MIXING {
        //     let [old_w2, old_w7, old_w15, old_w16] = [w[i - 2], w[i - 7], w[i - 15], w[i - 16]];
        //     let new_w = compute_w(old_w2, old_w7, old_w15, old_w16);
        //     let [old_a, old_b, old_c, old_d, old_e, old_f, old_g, old_h] = prev_state;
        //     let (a, e) = compute_ae(
        //         old_a,
        //         old_b,
        //         old_c,
        //         old_d,
        //         old_e,
        //         old_f,
        //         old_g,
        //         old_h,
        //         new_w,
        //         RC[CLOCKS_LOAD_INPUT + i],
        //     );

        //     let pos = offset + i;
        //     for j in 0..32 {
        //         let bit_a = ((a >> j) & 1) as u8;
        //         let bit_e = ((e >> j) & 1) as u8;
        //         let bit_w = ((new_w >> j) & 1) as u8;
        //         trace[pos].a[j] = F::from_u8(bit_a);
        //         trace[pos].e[j] = F::from_u8(bit_e);
        //         trace[pos].w[j] = F::from_u8(bit_w);
        //     }

        //     // Update prev_state for the next iteration
        //     prev_state[7] = old_g;
        //     prev_state[6] = old_f;
        //     prev_state[5] = old_e;
        //     prev_state[4] = e;
        //     prev_state[3] = old_c;
        //     prev_state[2] = old_b;
        //     prev_state[1] = old_a;
        //     prev_state[0] = a;

        //     // Update the w array for the next iteration
        //     for j in 0..15 {
        //         w[j] = w[j + 1];
        //     }
        //     w[15] = new_w;
        // }

        fn pack32(x: &[u8; 32]) -> u32 {
            let mut result = 0u32;
            for i in 0..32 {
                result |= (x[i] as u32) << (32 - 1 - i);
            }
            result
        }

        fn compute_ae(
            old_a: u32,
            old_b: u32,
            old_c: u32,
            old_d: u32,
            old_e: u32,
            old_f: u32,
            old_g: u32,
            old_h: u32,
            w: u32,
            k: u32,
        ) -> (u32, u32) {
            let s0 = rotate_right(old_a, 2) ^ rotate_right(old_a, 13) ^ rotate_right(old_a, 22);
            let s1 = rotate_right(old_e, 6) ^ rotate_right(old_e, 11) ^ rotate_right(old_e, 25);
            let t1 = old_h
                .wrapping_add(s1)
                .wrapping_add(ch(old_e, old_f, old_g))
                .wrapping_add(k)
                .wrapping_add(w);
            let t2 = s0.wrapping_add(maj(old_a, old_b, old_c));
            let a = t1.wrapping_add(t2);
            let e = old_d.wrapping_add(t1);
            (a, e)
        }

        fn compute_w(old_w2: u32, old_w7: u32, old_w15: u32, old_w16: u32) -> u32 {
            let s0 = rotate_right(old_w15, 7) ^ rotate_right(old_w15, 18) ^ shift_right(old_w15, 3);
            let s1 = rotate_right(old_w2, 17) ^ rotate_right(old_w2, 19) ^ shift_right(old_w2, 10);
            s1.wrapping_add(old_w7).wrapping_add(s0).wrapping_add(old_w16)
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
    pub fn compute_witness<F: PrimeField64>(&self, inputs: &[Vec<Sha256fInput>]) -> AirInstance<F> {
        // Get the fixed cols
        let num_rows = Sha256fDirectTrace::<F>::NUM_ROWS;
        let num_available_sha256fs = self.num_available_sha256fs;

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
        let mut sha256f_trace = Sha256fDirectTrace::new_zeroes();

        // Fill the the trace
        let mut index = 0;
        for inputs in inputs.iter() {
            for input in inputs.iter() {
                let row_offset = index * CLOCKS;
                self.process_sha256f(input, &mut sha256f_trace, row_offset);
                index += 1;
            }
        }
        timer_stop_and_log_trace!(SHA256F_TRACE);

        timer_start_trace!(SHA256F_PADDING);
        let padding_row = Sha256fDirectTraceRow::<F> { ..Default::default() };
        sha256f_trace.buffer[num_rows_filled..num_rows].fill(padding_row);
        timer_stop_and_log_trace!(SHA256F_PADDING);

        AirInstance::new_from_trace(FromTrace::new(&mut sha256f_trace))
    }
}
