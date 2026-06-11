use core::panic;
use std::sync::Arc;

use fields::PrimeField64;
use rayon::prelude::*;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::OperationBlake2Data;
use zisk_pil::{Blake2brTrace, Blake2brTraceRow, Blake2brTraceRowOps};

use super::blake2_constants::{CLOCKS, CLOCKS_PER_G, R1_G, R2_G, R3_G, R4_G, SIGMA};

/// Per-operation input record assembled from the bus payload.
#[derive(Debug)]
pub struct Blake2Input {
    pub addr_main: u32,
    pub step_main: u64,
    pub index: u64,
    pub state_addr: u32,
    pub input_addr: u32,
    pub state: [u64; 16],
    pub input: [u64; 16],
}

impl Blake2Input {
    pub fn from(values: &OperationBlake2Data<u64>) -> Self {
        Self {
            addr_main: values[3] as u32,
            step_main: values[4],
            index: values[5],
            state_addr: values[6] as u32,
            input_addr: values[7] as u32,
            state: values[8..24].try_into().unwrap(),
            input: values[24..40].try_into().unwrap(),
        }
    }
}

/// The `Blake2SM` struct encapsulates the logic of the Blake2 State Machine.
pub struct Blake2SM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Number of available blake2s in the trace.
    pub num_available_blake2s: usize,

    num_non_usable_rows: usize,

    range_id: usize,
}

impl<F: PrimeField64> Blake2SM<F> {
    /// Creates a new Blake2 State Machine instance.
    ///
    /// # Returns
    /// A new `Blake2SM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Compute some useful values
        let num_non_usable_rows = Blake2brTrace::<Blake2brTraceRow<F>>::NUM_ROWS % CLOCKS;
        let num_available_blake2s = Blake2brTrace::<Blake2brTraceRow<F>>::NUM_ROWS / CLOCKS
            - (num_non_usable_rows != 0) as usize;

        let range_id = std.get_range_id(0, (1 << 16) - 1, None).expect("Failed to get range ID");

        Arc::new(Self { std, num_available_blake2s, num_non_usable_rows, range_id })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Blake2 trace.
    /// * `num_circuits` - The number of circuits to process.
    /// * `input` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    #[inline(always)]
    pub fn process_input<R: Blake2brTraceRowOps<F>>(
        &self,
        input: &Blake2Input,
        trace: &mut [R],
    ) -> [u32; 65536] {
        let mut range_checks = [0u32; 65536]; // 2^16 range checks for the 16-bit limbs

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

            // Set idx_sel
            row.set_round_idx_sel(idx_usize, true);
        }

        // Set m columns
        let mut m_idx = 0;
        for (i, &inp) in input.iter().enumerate() {
            let low_inp = [inp as u16, (inp >> 16) as u16];
            let high_inp = [(inp >> 32) as u16, (inp >> 48) as u16];
            trace[m_idx].set_all_m_limbs(&[low_inp, high_inp]);
            range_checks[low_inp[0] as usize] += 1;
            range_checks[low_inp[1] as usize] += 1;
            range_checks[high_inp[0] as usize] += 1;
            range_checks[high_inp[1] as usize] += 1;

            m_idx += 1;
            if (i + 1) % (CLOCKS_PER_G - 1) == 0 {
                m_idx += 1;
            }
        }

        // Set ms columns
        let s = &SIGMA[idx_usize];
        let mut ms: [u64; 16] = [0u64; 16];
        m_idx = 0;
        for i in 0..input.len() {
            let inp = input[s[i]];
            ms[i] = inp;

            trace[m_idx].set_all_ms(&[inp as u32, (inp >> 32) as u32]);
            m_idx += 1;
            if (i + 1) % (CLOCKS_PER_G - 1) == 0 {
                m_idx += 1;
            }
        }

        // Column mixing
        let (state0, state4, state8, state12) = compute_g_and_set_vs::<R, F>(
            trace,
            &mut range_checks,
            0,
            &[state[0], state[4], state[8], state[12]],
            &[ms[0], ms[1]],
        );
        let (state1, state5, state9, state13) = compute_g_and_set_vs::<R, F>(
            trace,
            &mut range_checks,
            1,
            &[state[1], state[5], state[9], state[13]],
            &[ms[2], ms[3]],
        );
        let (state2, state6, state10, state14) = compute_g_and_set_vs::<R, F>(
            trace,
            &mut range_checks,
            2,
            &[state[2], state[6], state[10], state[14]],
            &[ms[4], ms[5]],
        );
        let (state3, state7, state11, state15) = compute_g_and_set_vs::<R, F>(
            trace,
            &mut range_checks,
            3,
            &[state[3], state[7], state[11], state[15]],
            &[ms[6], ms[7]],
        );

        // Diagonal mixing
        compute_g_and_set_vs::<R, F>(
            trace,
            &mut range_checks,
            4,
            &[state0, state5, state10, state15],
            &[ms[8], ms[9]],
        );
        compute_g_and_set_vs::<R, F>(
            trace,
            &mut range_checks,
            5,
            &[state1, state6, state11, state12],
            &[ms[10], ms[11]],
        );
        compute_g_and_set_vs::<R, F>(
            trace,
            &mut range_checks,
            6,
            &[state2, state7, state8, state13],
            &[ms[12], ms[13]],
        );
        compute_g_and_set_vs::<R, F>(
            trace,
            &mut range_checks,
            7,
            &[state3, state4, state9, state14],
            &[ms[14], ms[15]],
        );

        return range_checks;

        fn compute_g_and_set_vs<R: Blake2brTraceRowOps<F>, F: PrimeField64>(
            trace: &mut [R],
            range_checks: &mut [u32; 65536],
            i: usize,
            v: &[u64; 4],
            m: &[u64; 2],
        ) -> (u64, u64, u64, u64) {
            // Compute the g function
            let (va, vb, vc, vd) = (v[0], v[1], v[2], v[3]);
            let (va_i, vb_i, vc_i, vd_i) = compute_half_g(va, vb, vc, vd, m[0], R1_G, R2_G);
            let (va_o, vb_o, vc_o, vd_o) = compute_half_g(va_i, vb_i, vc_i, vd_i, m[1], R3_G, R4_G);

            // Set va, vb, vc, vd columns
            set_vs::<R, F>(&mut trace[3 * i], range_checks, va, vb, vc, vd);
            set_vs::<R, F>(&mut trace[3 * i + 1], range_checks, va_i, vb_i, vc_i, vd_i);
            set_vs::<R, F>(&mut trace[3 * i + 2], range_checks, va_o, vb_o, vc_o, vd_o);

            (va_o, vb_o, vc_o, vd_o)
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

        fn set_vs<R: Blake2brTraceRowOps<F>, F: PrimeField64>(
            row: &mut R,
            range_checks: &mut [u32; 65536],
            va: u64,
            vb: u64,
            vc: u64,
            vd: u64,
        ) {
            let low_va = [va as u16, (va >> 16) as u16];
            let high_va = [(va >> 32) as u16, (va >> 48) as u16];
            row.set_all_va_limbs(&[low_va, high_va]);
            range_checks[low_va[0] as usize] += 1;
            range_checks[low_va[1] as usize] += 1;
            range_checks[high_va[0] as usize] += 1;
            range_checks[high_va[1] as usize] += 1;

            let low_vb = u32_to_le_bits(vb as u32);
            let high_vb = u32_to_le_bits((vb >> 32) as u32);
            row.set_all_vb(&[low_vb, high_vb]);

            let low_vc = [vc as u16, (vc >> 16) as u16];
            let high_vc = [(vc >> 32) as u16, (vc >> 48) as u16];
            row.set_all_vc_limbs(&[low_vc, high_vc]);
            range_checks[low_vc[0] as usize] += 1;
            range_checks[low_vc[1] as usize] += 1;
            range_checks[high_vc[0] as usize] += 1;
            range_checks[high_vc[1] as usize] += 1;

            let low_vd = u32_to_le_bits(vd as u32);
            let high_vd = u32_to_le_bits((vd >> 32) as u32);
            row.set_all_vd(&[low_vd, high_vd]);
        }

        fn u32_to_le_bits(x: u32) -> [bool; 32] {
            let mut bits = [false; 32];
            for (i, bit) in bits.iter_mut().enumerate() {
                if ((x >> i) & 1) != 0 {
                    *bit = true;
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
    pub fn compute_witness<R: Blake2brTraceRowOps<F>>(
        &self,
        inputs: &[Vec<Blake2Input>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = Blake2brTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();
        let num_available_blake2s = self.num_available_blake2s;

        // Check that we can fit all the blake2s in the trace
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        let all_ops_used = num_inputs == num_available_blake2s;
        let num_rows_filled = num_inputs * CLOCKS;
        let num_rows_needed = if num_inputs < num_available_blake2s {
            num_inputs * CLOCKS
        } else if all_ops_used {
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
        let mut trace_rows = trace.buffer.as_mut_slice();
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

        // Fill the trace and collect range checks
        let range_checks_vec: Vec<[u32; 65536]> = par_traces
            .into_par_iter()
            .enumerate()
            .map(|(index, trace)| {
                let input_index = inputs_indexes[index];
                let input = &inputs[input_index.0][input_index.1];
                self.process_input::<R>(input, trace)
            })
            .collect();

        // Aggregate all range checks
        let mut range_checks = vec![0; 65536];
        for rc in range_checks_vec {
            for i in 0..65536 {
                range_checks[i] += rc[i];
            }
        }

        timer_stop_and_log_trace!(BLAKE2_TRACE);

        let mut padding_row = R::default();
        // In the no-op rows, the `idx` should be the same as the previous one until the end
        // to make the constraint `(1 - CLK_0) * (idx - 'idx) === 0;` be satisfied
        // As a consequence, one should also set idx_sel
        if all_ops_used {
            let prev_idx = trace.buffer[num_rows_filled - 1].get_round_idx();
            padding_row.set_round_idx_sel(prev_idx as usize, true);
        }

        trace.buffer[num_rows_filled..num_rows].par_iter_mut().for_each(|slot| *slot = padding_row);

        // Perform the zero range checks
        let mut count_zeros = ((num_available_blake2s - num_inputs
            + (self.num_non_usable_rows != 0) as usize)
            * CLOCKS
            + self.num_non_usable_rows)
            * 12; // 12 range checked columns, 8 from va and vc, and 4 from m16
        count_zeros += 8 * num_inputs * 4; // m16 columns have one padding row per g function
                                           // and there are 8 g functions per blake2
        range_checks[0] += count_zeros as u32;

        self.std.range_check_ranged(self.range_id, None, &range_checks);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}
