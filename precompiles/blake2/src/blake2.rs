use core::panic;
use std::sync::Arc;

use fields::PrimeField64;
use rayon::prelude::*;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
#[cfg(feature = "packed")]
use zisk_pil::{Blake2TracePacked, Blake2TraceRowPacked};
#[cfg(not(feature = "packed"))]
use zisk_pil::{Blake2brTrace, Blake2brTraceRow};

#[cfg(feature = "packed")]
type Blake2TraceRowType<F> = Blake2TraceRowPacked<F>;
#[cfg(feature = "packed")]
type Blake2TraceType<F> = Blake2TracePacked<F>;

#[cfg(not(feature = "packed"))]
type Blake2TraceRowType<F> = Blake2brTraceRow<F>;
#[cfg(not(feature = "packed"))]
type Blake2TraceType<F> = Blake2brTrace<F>;

use super::{
    blake2_constants::{CLOCKS, CLOCKS_PER_G, R1_G, R2_G, R3_G, R4_G, SIGMA},
    Blake2Input,
};

/// The `Blake2SM` struct encapsulates the logic of the Blake2 State Machine.
pub struct Blake2SM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Number of available blake2s in the trace.
    pub num_available_blake2s: usize,

    num_non_usable_rows: usize,
}

impl<F: PrimeField64> Blake2SM<F> {
    /// Creates a new Blake2 State Machine instance.
    ///
    /// # Returns
    /// A new `Blake2SM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Compute some useful values
        let num_non_usable_rows = Blake2TraceType::<F>::NUM_ROWS % CLOCKS;
        let num_available_blake2s =
            Blake2TraceType::<F>::NUM_ROWS / CLOCKS - (num_non_usable_rows != 0) as usize;

        Arc::new(Self { std, num_available_blake2s, num_non_usable_rows })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Blake2 trace.
    /// * `num_circuits` - The number of circuits to process.
    /// * `input` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    #[inline(always)]
    pub fn process_input(&self, input: &Blake2Input, trace: &mut [Blake2TraceRowType<F>]) {
        let step_main = input.step_main;
        let addr_main = input.addr_main;
        let state_addr = input.state_addr;
        let input_addr = input.input_addr;
        let index = input.index as u8;
        let state = &input.state;
        let input = &input.input;

        // Fill the step_addr
        trace[0].set_step_addr(step_main); // STEP_MAIN
        trace[1].set_step_addr(addr_main as u64); // ADDR_OP
        trace[2].set_step_addr(state_addr as u64); // ADDR_STATE
        trace[3].set_step_addr(input_addr as u64); // ADDR_INPUT
        trace[4].set_step_addr(state_addr as u64); // ADDR_IND_0
        trace[5].set_step_addr(input_addr as u64); // ADDR_IND_1

        // Set latched columns
        let idx_usize = index as usize;
        for row in trace.iter_mut().take(CLOCKS) {
            // Activate the in_use selector
            row.set_in_use(true);

            // Set idx
            row.set_idx(index);

            // Set idx_sel
            row.set_idx_sel(idx_usize, true);
        }

        // Set m columns
        let mut m_idx = 0;
        for (i, &inp) in input.iter().enumerate() {
            trace[m_idx].set_m(0, inp as u32);
            trace[m_idx].set_m(1, (inp >> 32) as u32);
            m_idx += 1;
            if i % CLOCKS_PER_G == (CLOCKS_PER_G - 1) {
                m_idx += 1;
            }
        }

        // Set ms columns
        let s = &SIGMA[idx_usize];
        let mut ms: [u64; 16] = [0u64; 16];
        m_idx = 0;
        for i in 0..input.len() {
            ms[i] = input[s[i]];

            trace[m_idx].set_ms(0, trace[m_idx].get_m(0));
            trace[m_idx].set_ms(1, trace[m_idx].get_m(1));
            m_idx += 1;
            if i % CLOCKS_PER_G == (CLOCKS_PER_G - 1) {
                m_idx += 1;
            }
        }

        // Column mixing
        compute_g_and_set_vs(trace, 0, &[state[0], state[4], state[8], state[12]], &[ms[0], ms[1]]);
        compute_g_and_set_vs(trace, 1, &[state[1], state[5], state[9], state[13]], &[ms[2], ms[3]]);
        compute_g_and_set_vs(
            trace,
            2,
            &[state[2], state[6], state[10], state[14]],
            &[ms[4], ms[5]],
        );
        compute_g_and_set_vs(
            trace,
            3,
            &[state[3], state[7], state[11], state[15]],
            &[ms[6], ms[7]],
        );

        // Diagonal mixing
        compute_g_and_set_vs(
            trace,
            4,
            &[state[0], state[5], state[10], state[15]],
            &[ms[8], ms[9]],
        );
        compute_g_and_set_vs(
            trace,
            5,
            &[state[1], state[6], state[11], state[12]],
            &[ms[10], ms[11]],
        );
        compute_g_and_set_vs(
            trace,
            6,
            &[state[2], state[7], state[8], state[13]],
            &[ms[12], ms[13]],
        );
        compute_g_and_set_vs(
            trace,
            7,
            &[state[3], state[4], state[9], state[14]],
            &[ms[14], ms[15]],
        );

        fn compute_g_and_set_vs<F: PrimeField64>(
            trace: &mut [Blake2TraceRowType<F>],
            i: usize,
            v: &[u64; 4],
            m: &[u64; 2],
        ) {
            // Compute the g function
            let (va, vb, vc, vd) = (v[0], v[1], v[2], v[3]);
            let (va_i, vb_i, vc_i, vd_i) = compute_half_g(va, vb, vc, vd, m[0], R1_G, R2_G);
            let (va_o, vb_o, vc_o, vd_o) = compute_half_g(va_i, vb_i, vc_i, vd_i, m[1], R3_G, R4_G);

            // Set va, vb, vc, vd columns
            set_vs(&mut trace[3 * i], va, vb, vc, vd);
            set_vs(&mut trace[3 * i + 1], va_i, vb_i, vc_i, vd_i);
            set_vs(&mut trace[3 * i + 2], va_o, vb_o, vc_o, vd_o);
        }

        fn compute_half_g(
            va: u64,
            vb: u64,
            vc: u64,
            vd: u64,
            m: u64,
            r1: u32,
            r2: u32,
        ) -> (u64, u64, u64, u64) {
            let va = va.wrapping_add(vb).wrapping_add(m);
            let vd = (vd ^ va).rotate_right(r1);
            let vc = vc.wrapping_add(vd);
            let vb = (vb ^ vc).rotate_right(r2);
            (va, vb, vc, vd)
        }

        fn set_vs<F: PrimeField64>(
            row: &mut Blake2TraceRowType<F>,
            va: u64,
            vb: u64,
            vc: u64,
            vd: u64,
        ) {
            let low_va = va as u32;
            let high_va = (va >> 32) as u32;
            row.set_va(0, low_va);
            row.set_va(1, high_va);

            let low_vb = vb as u32;
            let low_vb = u32_to_le_bits(low_vb);
            let high_vb = (vb >> 32) as u32;
            let high_vb = u32_to_le_bits(high_vb);
            for j in 0..32 {
                row.set_vb(0, j, low_vb[j]);
                row.set_vb(1, j, high_vb[j]);
            }

            let low_vc = vc as u32;
            let high_vc = (vc >> 32) as u32;
            row.set_vc(0, low_vc);
            row.set_vc(1, high_vc);

            let low_vd = vd as u32;
            let low_vd = u32_to_le_bits(low_vd);
            let high_vd = (vd >> 32) as u32;
            let high_vd = u32_to_le_bits(high_vd);
            for j in 0..32 {
                row.set_vd(0, j, low_vd[j]);
                row.set_vd(1, j, high_vd[j]);
            }
        }

        fn u32_to_le_bits(x: u32) -> [bool; 32] {
            let mut bits = [false; 32];
            for i in 0..32 {
                if ((x >> i) & 1) != 0 {
                    bits[i] = true;
                }
            }
            bits
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
        inputs: &[Vec<Blake2Input>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut blake2_trace = Blake2TraceType::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = blake2_trace.num_rows();
        let num_available_blake2s = self.num_available_blake2s;

        // Check that we can fit all the blake2s in the trace
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        let num_rows_filled = num_inputs * CLOCKS;
        let num_rows_needed = if num_inputs < num_available_blake2s {
            num_inputs * CLOCKS
        } else if num_inputs == num_available_blake2s {
            num_rows
        } else {
            panic!(
                "Exceeded available Blake2s inputs: requested {}, but only {} are available.",
                num_inputs, self.num_available_blake2s
            );
        };

        tracing::debug!(
            "··· Creating Blake2 instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(BLAKE2_TRACE);

        // Split trace into chunks for parallel processing
        let mut trace_rows = blake2_trace.buffer.as_mut_slice();
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
        par_traces.into_par_iter().enumerate().for_each(|(index, trace)| {
            let input_index = inputs_indexes[index];
            let input = &inputs[input_index.0][input_index.1];
            self.process_input(input, trace)
        });

        timer_stop_and_log_trace!(BLAKE2_TRACE);

        timer_start_trace!(BLAKE2_PADDING);
        // // Set a = e = w = 0 for the state and input rows
        // let zero_row = Blake2TraceRowType::<F>::default();

        // let padding_start = num_rows_filled;
        // let padding_end = num_rows - self.num_non_usable_rows - CLOCKS;

        // if padding_start < padding_end {
        //     blake2_trace.buffer[padding_start..padding_end]
        //         .par_iter_mut()
        //         .for_each(|row| {
        //             *row = zero_row;
        //         });
        // }

        timer_stop_and_log_trace!(BLAKE2_PADDING);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut blake2_trace)))
    }
}
