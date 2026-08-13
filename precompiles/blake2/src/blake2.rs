use core::panic;
use std::sync::Arc;

use proofman_fields::PrimeField64;
use rayon::prelude::*;

use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::OperationBlake2Data;
use zisk_pil::{Blake2brTrace, Blake2brTraceRow, Blake2brTraceRowOps};

use super::blake2_constants::{BLAKE2BR_TABLE_SIZE, CLOCKS, R1_G, R2_G, R3_G, R4_G, SIGMA};
use super::blake2_table::Blake2brTableSM;

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

/// Number of 16-bit range-checked limbs per row: x[4], y[4], va[4], vc[4].
const RANGE_CHECKED_LIMBS_PER_ROW: usize = 16;

/// Number of unconditional XOR table lookups per row:
/// vd', vb', vd'' and vb_pp_xor, 8 bytes each.
const XOR_CHECKS_PER_ROW: usize = 32;

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

    range_id: usize,

    table_id: usize,
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

        let table_id = std
            .get_virtual_table_id(Blake2brTableSM::TABLE_ID)
            .expect("Failed to get Blake2br table ID");

        Arc::new(Self { std, num_available_blake2s, range_id, table_id })
    }

    /// Processes one operation, filling its CLOCKS-row chunk of the trace and
    /// updating the range-check and XOR-table multiplicities.
    ///
    /// # Arguments
    /// * `input` - The operation data to process.
    /// * `trace` - The CLOCKS-row chunk of the trace assigned to this operation.
    /// * `range_checks` - Multiplicities of the 16-bit range checks.
    /// * `xor_checks` - Multiplicities of the Blake2br XOR table rows.
    #[inline(always)]
    pub fn process_input<R: Blake2brTraceRowOps<F>>(
        &self,
        input: &Blake2Input,
        trace: &mut [R],
        range_checks: &mut [u32],
        xor_checks: &mut [u32],
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
            for limb in x_limbs {
                range_checks[limb as usize] += 1;
            }
            for limb in y_limbs {
                range_checks[limb as usize] += 1;
            }

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
            for limb in va_limbs {
                range_checks[limb as usize] += 1;
            }
            for limb in vc_limbs {
                range_checks[limb as usize] += 1;
            }

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

            // XOR table lookups: (vd, va'), (vb, vc'), (vd', va'') and (vb', vc''), per byte
            for i in 0..8 {
                let rows = [
                    Blake2brTableSM::calculate_table_row(vd_bytes[i], va_p_bytes[i]),
                    Blake2brTableSM::calculate_table_row(vb_bytes[i], vc_p_bytes[i]),
                    Blake2brTableSM::calculate_table_row(vd_p_bytes[i], va_pp_bytes[i]),
                    Blake2brTableSM::calculate_table_row(vb_p_bytes[i], vc_pp_bytes[i]),
                ];
                for table_row in rows {
                    xor_checks[table_row as usize] += 1;
                }
            }

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
    pub fn compute_witness<R: Blake2brTraceRowOps<F>>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<Blake2Input>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = Blake2brTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();
        let num_available_blake2s = self.num_available_blake2s;

        // Check that we can fit all the blake2s in the trace
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        if num_inputs > num_available_blake2s {
            panic!(
                "Exceeded available Blake2s inputs: requested {}, but only {} are available.",
                num_inputs, num_available_blake2s
            );
        }
        let num_rows_filled = num_inputs * CLOCKS;

        tracing::debug!(
            "··· Creating Blake2 instance [{} / {} rows filled {:.2}%]",
            num_rows_filled,
            num_rows,
            num_rows_filled as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(BLAKE2_TRACE);

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

        // Fill the trace, collecting the range-check and XOR-table multiplicities
        let (mut range_checks, xor_checks) = par_traces
            .into_par_iter()
            .enumerate()
            .fold(
                || (vec![0u32; 1 << 16], vec![0u32; BLAKE2BR_TABLE_SIZE]),
                |(mut range_checks, mut xor_checks), (index, trace)| {
                    let input_index = inputs_indexes[index];
                    let input = &inputs[input_index.0][input_index.1];
                    self.process_input::<R>(input, trace, &mut range_checks, &mut xor_checks);
                    (range_checks, xor_checks)
                },
            )
            .reduce(
                || (vec![0u32; 1 << 16], vec![0u32; BLAKE2BR_TABLE_SIZE]),
                |(mut range_acc, mut xor_acc), (range, xor)| {
                    for (acc, val) in range_acc.iter_mut().zip(range) {
                        *acc += val;
                    }
                    for (acc, val) in xor_acc.iter_mut().zip(xor) {
                        *acc += val;
                    }
                    (range_acc, xor_acc)
                },
            );

        // Padding rows are all-zero: in_use is off, so the only bus contributions
        // are the unconditional range checks and XOR table lookups over zeros
        trace.buffer[num_rows_filled..num_rows]
            .par_iter_mut()
            .for_each(|slot| *slot = R::default());

        let num_padding_rows = (num_rows - num_rows_filled) as u32;
        range_checks[0] += RANGE_CHECKED_LIMBS_PER_ROW as u32 * num_padding_rows;

        timer_stop_and_log_trace!(BLAKE2_TRACE);

        self.std.range_check_ranged(self.range_id, None, &range_checks);

        let zero_row = Blake2brTableSM::calculate_table_row(0, 0) as usize;
        xor_checks.into_par_iter().enumerate().for_each(|(row, mut value)| {
            if row == zero_row {
                value += XOR_CHECKS_PER_ROW as u32 * num_padding_rows;
            }
            if value > 0 {
                self.std.inc_virtual_row(self.table_id, row as u32, value);
            }
        });

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}
