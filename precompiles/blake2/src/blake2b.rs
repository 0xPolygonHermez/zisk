use core::panic;
use std::sync::Arc;
use core::marker::PhantomData;
use proofman_fields::PrimeField64;
use rayon::prelude::*;

use proofman_common::{AirInstance, FromTrace, GenericTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::OperationBlake2bData;
use zisk_pil::{Blake2brTraceRowOps, ZISK_AIRGROUP_ID};

use super::blake2b_constants::{CLOCKS, R1_G, R2_G, R3_G, R4_G, SIGMA};

/// State indices (a, b, c, d) mixed by the G function at each clock:
/// clocks 0-3 perform the column mixing, clocks 4-7 the diagonal mixing.
const G_INDICES: [(usize, usize, usize, usize); CLOCKS] = [
    (0, 4, 8, 12),
    (1, 5, 9, 13),
    (2, 6, 10, 14),
    (3, 7, 11, 15),
    (0, 5, 10, 15),
    (1, 6, 11, 12),
    (2, 7, 8, 13),
    (3, 4, 9, 14),
];


/// Per-operation input record assembled from the bus payload.
#[derive(Debug)]
pub struct Blake2bInput {
    pub addr_main: u32,
    pub step_main: u64,
    pub index: u64,
    pub state_addr: u32,
    pub input_addr: u32,
    pub state: [u64; 16],
    pub input: [u64; 16],
}

impl Blake2bInput {
    pub fn from(values: &OperationBlake2bData<u64>) -> Self {
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

/// The `Blake2bSM` struct encapsulates the logic of the Blake2b State Machine.
/// Nothing here depends on the height of the air: the capacity is taken from the trace each call
/// builds, so a taller sibling would need no change.
pub struct Blake2bSM<F: PrimeField64> {
    /// Phantom data to hold the generic type F.
    ph: PhantomData<F>,
}

impl<F: PrimeField64> Blake2bSM<F> {
    /// Creates a new Blake2b State Machine instance.
    ///
    /// # Returns
    /// A new `Blake2bSM` instance.
    pub fn new() -> Arc<Self> {
        // The blake table (129) is counted by the prover now, so there is nothing to hold here
        // beyond the generic marker.
        Arc::new(Self { ph: PhantomData })
    }

    /// Processes one operation, filling its CLOCKS-row chunk of the trace.
    ///
    /// # Arguments
    /// * `input` - The operation data to process.
    /// * `trace` - The CLOCKS-row chunk of the trace assigned to this operation.
    #[inline(always)]
    pub fn process_input<R: Blake2brTraceRowOps<F>>(
        &self,
        input: &Blake2bInput,
        trace: &mut [R],
    ) {
        let idx_usize = input.index as usize;
        let s = &SIGMA[idx_usize];

        // Fill the step_addr
        trace[0].set_step_addr(input.step_main); // STEP_MAIN
        trace[1].set_step_addr(input.addr_main as u64); // ADDR_OP
        trace[2].set_step_addr(input.state_addr as u64); // ADDR_STATE
        trace[3].set_step_addr(input.input_addr as u64); // ADDR_INPUT
        trace[4].set_step_addr(input.state_addr as u64); // ADDR_IND_0
        trace[5].set_step_addr(input.input_addr as u64); // ADDR_IND_1

        // Running state: each row's G function reads and writes 4 words of it
        let mut v = input.state;

        for (k, row) in trace.iter_mut().enumerate().take(CLOCKS) {
            row.set_in_use(true);
            row.set_round_idx_sel(idx_usize, true);

            // Memory-ordered message words bound by the x/y memory ports at this clock
            let x_limbs = u64_to_limbs16(input.input[2 * k]);
            let y_limbs = u64_to_limbs16(input.input[2 * k + 1]);
            row.set_all_x(&x_limbs);
            row.set_all_y(&y_limbs);

            // Permuted message words consumed by this row's G function
            let xs = input.input[s[2 * k]];
            let ys = input.input[s[2 * k + 1]];
            row.set_all_xs(&[xs as u32, (xs >> 32) as u32]);
            row.set_all_ys(&[ys as u32, (ys >> 32) as u32]);

            // Compute the G function
            let (ia, ib, ic, id) = G_INDICES[k];
            let (va, vb, vc, vd) = (v[ia], v[ib], v[ic], v[id]);

            let va_p = va.wrapping_add(vb).wrapping_add(xs);
            let vd_p = (vd ^ va_p).rotate_right(R1_G);
            let vc_p = vc.wrapping_add(vd_p);
            let vb_p = (vb ^ vc_p).rotate_right(R2_G);
            let va_pp = va_p.wrapping_add(vb_p).wrapping_add(ys);
            let vd_pp = (vd_p ^ va_pp).rotate_right(R3_G);
            let vc_pp = vc_p.wrapping_add(vd_pp);
            let z = vb_p ^ vc_pp;
            let vb_pp = z.rotate_right(R4_G);

            // Inputs: va/vc as 16-bit limbs (range checked), vb/vd as bytes
            let va_limbs = u64_to_limbs16(va);
            let vc_limbs = u64_to_limbs16(vc);
            row.set_all_va(&va_limbs);
            row.set_all_vc(&vc_limbs);

            let vb_bytes = vb.to_le_bytes();
            let vd_bytes = vd.to_le_bytes();
            row.set_all_vb(&vb_bytes);
            row.set_all_vd(&vd_bytes);

            // Intermediate and output values as bytes
            let va_p_bytes = va_p.to_le_bytes();
            let vd_p_bytes = vd_p.to_le_bytes();
            let vc_p_bytes = vc_p.to_le_bytes();
            let vb_p_bytes = vb_p.to_le_bytes();
            let va_pp_bytes = va_pp.to_le_bytes();
            let vd_pp_bytes = vd_pp.to_le_bytes();
            let vc_pp_bytes = vc_pp.to_le_bytes();
            let z_bytes = z.to_le_bytes();
            row.set_all_va_prime(&va_p_bytes);
            row.set_all_vd_prime(&vd_p_bytes);
            row.set_all_vc_prime(&vc_p_bytes);
            row.set_all_vb_prime(&vb_p_bytes);
            row.set_all_va_prime_prime(&va_pp_bytes);
            row.set_all_vd_prime_prime(&vd_pp_bytes);
            row.set_all_vc_prime_prime(&vc_pp_bytes);
            row.set_all_vb_pp_xor(&z_bytes);

            // Top bits of z's low and high 32-bit limbs (rotl-by-1 carries)
            row.set_all_vb_pp_t(&[(z >> 31) & 1 == 1, (z >> 63) & 1 == 1]);

            // Write the outputs back for the following rows
            v[ia] = va_pp;
            v[ib] = vb_pp;
            v[ic] = vc_pp;
            v[id] = vd_pp;
        }

        fn u64_to_limbs16(value: u64) -> [u16; 4] {
            [value as u16, (value >> 16) as u16, (value >> 32) as u16, (value >> 48) as u16]
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
    /// The air is selected by the `NUM_ROWS` / `AIR_ID` consts of the trace this builds, so one
    /// body serves every height the air is instantiated at.
    pub fn compute_witness<
        R: Blake2brTraceRowOps<F>,
        const NUM_ROWS: usize,
        const AIR_ID: usize,
    >(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<Blake2bInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = GenericTrace::<R, NUM_ROWS, ZISK_AIRGROUP_ID, AIR_ID>::new_from_vec_zeroes(
            trace_buffer,
        )?;
        let num_rows = trace.num_rows();
        // Capacity of the air this call builds, taken from `NUM_ROWS`: deriving it from a
        // fixed trace alias instead is what breaks the moment the air gains a taller
        // sibling, since the instance would be measured against the short air's capacity.
        //
        // Plain floor division, which is what the PIL commits to (`NUM_OPS = (N - N % CLOCKS) /
        // CLOCKS` in blake2br.pil) and what the planner advertises through `num_available` in
        // `lib.rs`. CLOCKS is 8 and NUM_ROWS a power of two, so the division is exact and an
        // extra `- (NUM_ROWS % CLOCKS != 0)` term used to be invisible here -- it was the same
        // expression that cost Blake2s its last operation, where 1048576 / 80 leaves 16 rows over.
        // Keeping it a plain division means a future change to either constant cannot revive that.
        let num_available_blake2bs = NUM_ROWS / CLOCKS;

        // Check that we can fit all the blake2b rounds in the trace
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        if num_inputs > num_available_blake2bs {
            panic!(
                "Exceeded available Blake2b inputs: requested {}, but only {} are available.",
                num_inputs, num_available_blake2bs
            );
        }
        let num_rows_filled = num_inputs * CLOCKS;

        tracing::debug!(
            "··· Creating Blake2b instance [{} / {} rows filled {:.2}%]",
            num_rows_filled,
            num_rows,
            num_rows_filled as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(BLAKE2B_TRACE);

        // Split trace into per-operation chunks for parallel processing
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

        // Fill the trace
        par_traces
            .into_par_iter()
            .enumerate()
            .for_each(
                |(index, trace)| {
                    let input_index = inputs_indexes[index];
                    let input = &inputs[input_index.0][input_index.1];
                    self.process_input::<R>(input, trace);
                },
            );

        // Padding rows are all-zero: in_use is off, so the only bus contributions
        // are the unconditional range checks and XOR table lookups over zeros
        trace.buffer[num_rows_filled..num_rows]
            .par_iter_mut()
            .for_each(|slot| *slot = R::default());

        timer_stop_and_log_trace!(BLAKE2B_TRACE);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}
