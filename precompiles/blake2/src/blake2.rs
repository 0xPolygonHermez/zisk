use core::panic;
use std::sync::Arc;

use fields::PrimeField64;
use rayon::prelude::*;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
#[cfg(not(feature = "packed"))]
use zisk_pil::{Blake2brTrace, Blake2brTraceRow};
#[cfg(feature = "packed")]
use zisk_pil::{Blake2brTracePacked, Blake2brTraceRowPacked};
#[cfg(feature = "packed")]
type Blake2TraceRowType<F> = Blake2brTraceRowPacked<F>;
#[cfg(feature = "packed")]
type Blake2TraceType<F> = Blake2brTracePacked<F>;

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

    range_id: usize,
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
    pub fn process_input(
        &self,
        input: &Blake2Input,
        trace: &mut [Blake2TraceRowType<F>],
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

            // Set idx
            row.set_round_idx(index);

            // Set idx_sel
            row.set_round_idx_sel(idx_usize, true);
        }

        // Set m columns
        let mut m_idx = 0;
        for (i, &inp) in input.iter().enumerate() {
            let low_inp = [inp as u16, (inp >> 16) as u16];
            let high_inp = [(inp >> 32) as u16, (inp >> 48) as u16];
            for j in 0..2 {
                trace[m_idx].set_m_limbs(0, j, low_inp[j]);
                trace[m_idx].set_m_limbs(1, j, high_inp[j]);
            }
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

            trace[m_idx].set_ms(0, inp as u32);
            trace[m_idx].set_ms(1, (inp >> 32) as u32);
            m_idx += 1;
            if (i + 1) % (CLOCKS_PER_G - 1) == 0 {
                m_idx += 1;
            }
        }

        // Column mixing
        let (state0, state4, state8, state12) = compute_g_and_set_vs(
            trace,
            &mut range_checks,
            0,
            &[state[0], state[4], state[8], state[12]],
            &[ms[0], ms[1]],
        );
        let (state1, state5, state9, state13) = compute_g_and_set_vs(
            trace,
            &mut range_checks,
            1,
            &[state[1], state[5], state[9], state[13]],
            &[ms[2], ms[3]],
        );
        let (state2, state6, state10, state14) = compute_g_and_set_vs(
            trace,
            &mut range_checks,
            2,
            &[state[2], state[6], state[10], state[14]],
            &[ms[4], ms[5]],
        );
        let (state3, state7, state11, state15) = compute_g_and_set_vs(
            trace,
            &mut range_checks,
            3,
            &[state[3], state[7], state[11], state[15]],
            &[ms[6], ms[7]],
        );

        // Diagonal mixing
        compute_g_and_set_vs(
            trace,
            &mut range_checks,
            4,
            &[state0, state5, state10, state15],
            &[ms[8], ms[9]],
        );
        compute_g_and_set_vs(
            trace,
            &mut range_checks,
            5,
            &[state1, state6, state11, state12],
            &[ms[10], ms[11]],
        );
        compute_g_and_set_vs(
            trace,
            &mut range_checks,
            6,
            &[state2, state7, state8, state13],
            &[ms[12], ms[13]],
        );
        compute_g_and_set_vs(
            trace,
            &mut range_checks,
            7,
            &[state3, state4, state9, state14],
            &[ms[14], ms[15]],
        );

        return range_checks;

        fn compute_g_and_set_vs<F: PrimeField64>(
            trace: &mut [Blake2TraceRowType<F>],
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
            set_vs(&mut trace[3 * i], range_checks, va, vb, vc, vd);
            set_vs(&mut trace[3 * i + 1], range_checks, va_i, vb_i, vc_i, vd_i);
            set_vs(&mut trace[3 * i + 2], range_checks, va_o, vb_o, vc_o, vd_o);

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

        fn set_vs<F: PrimeField64>(
            row: &mut Blake2TraceRowType<F>,
            range_checks: &mut [u32; 65536],
            va: u64,
            vb: u64,
            vc: u64,
            vd: u64,
        ) {
            let low_va = [va as u16, (va >> 16) as u16];
            let high_va = [(va >> 32) as u16, (va >> 48) as u16];
            for j in 0..2 {
                row.set_va_limbs(0, j, low_va[j]);
                row.set_va_limbs(1, j, high_va[j]);
            }
            range_checks[low_va[0] as usize] += 1;
            range_checks[low_va[1] as usize] += 1;
            range_checks[high_va[0] as usize] += 1;
            range_checks[high_va[1] as usize] += 1;

            let low_vb = vb as u32;
            let low_vb = u32_to_le_bits(low_vb);
            let high_vb = (vb >> 32) as u32;
            let high_vb = u32_to_le_bits(high_vb);
            for j in 0..32 {
                row.set_vb(0, j, low_vb[j]);
                row.set_vb(1, j, high_vb[j]);
            }

            let low_vc = [vc as u16, (vc >> 16) as u16];
            let high_vc = [(vc >> 32) as u16, (vc >> 48) as u16];
            for j in 0..2 {
                row.set_vc_limbs(0, j, low_vc[j]);
                row.set_vc_limbs(1, j, high_vc[j]);
            }
            range_checks[low_vc[0] as usize] += 1;
            range_checks[low_vc[1] as usize] += 1;
            range_checks[high_vc[0] as usize] += 1;
            range_checks[high_vc[1] as usize] += 1;

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
    pub fn compute_witness(
        &self,
        inputs: &[Vec<Blake2Input>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = Blake2TraceType::new_from_vec_zeroes(trace_buffer)?;
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
                self.process_input(input, trace)
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

        let mut padding_row = Blake2TraceRowType::default();
        // In the no-op rows, the `idx` should be the same as the previous one until the end
        // to make the constraint `(1 - CLK_0) * (idx - 'idx) === 0;` be satisfied
        // As a consequence, one should also set idx_sel
        if all_ops_used {
            let prev_idx = trace.buffer[num_rows_filled - 1].get_round_idx();
            padding_row.set_round_idx(prev_idx);
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

        self.std.range_checks(self.range_id, range_checks);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}
