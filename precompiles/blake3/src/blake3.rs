use core::panic;
use std::sync::Arc;

use proofman_fields::PrimeField64;
use rayon::prelude::*;

use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::OperationBlake3Data;
use zisk_pil::{Blake3fTrace, Blake3fTraceRow, Blake3fTraceRowOps};

use super::blake3_constants::{
    BLAKE3F_TABLE_SIZE, CLOCKS, NUM_G_PER_ROUND, R1_G, R2_G, R3_G, R4_G, SIGMA,
};
use super::blake3_table::Blake3fTableSM;

/// State indices (a, b, c, d) mixed by the G function at each clock of a round:
/// clocks 0-3 perform the column mixing, clocks 4-7 the diagonal mixing.
const G_INDICES: [(usize, usize, usize, usize); NUM_G_PER_ROUND] = [
    (0, 4, 8, 12),
    (1, 5, 9, 13),
    (2, 6, 10, 14),
    (3, 7, 11, 15),
    (0, 5, 10, 15),
    (1, 6, 11, 12),
    (2, 7, 8, 13),
    (3, 4, 9, 14),
];

/// The AIR is templated over LANES side-by-side operations per row;
/// the current instance is compiled with LANES = 1.
const LANE: usize = 0;

/// Number of 16-bit range-checked limbs per row (per lane): va[2], vc[2], x[2], y[2].
const RANGE_CHECKED_LIMBS_PER_ROW: usize = 8;

/// Number of unconditional rot-0 XOR table lookups per row (per lane):
/// vd', vd'' and vb''-xor, 4 bytes each.
const XOR_ROT0_CHECKS_PER_ROW: usize = 12;

/// Number of unconditional rot-12 XOR table lookups per row (per lane): vb', 4 bytes.
const XOR_ROT12_CHECKS_PER_ROW: usize = 4;

/// Per-operation input record assembled from the bus payload.
#[derive(Debug)]
pub struct Blake3Input {
    pub addr_main: u32,
    pub step_main: u64,
    pub state_addr: u32,
    pub input_addr: u32,
    pub state: [u64; 8],
    pub input: [u64; 8],
}

impl Blake3Input {
    pub fn from(values: &OperationBlake3Data<u64>) -> Self {
        Self {
            addr_main: values[3] as u32,
            step_main: values[4],
            state_addr: values[5] as u32,
            input_addr: values[6] as u32,
            state: values[7..15].try_into().unwrap(),
            input: values[15..23].try_into().unwrap(),
        }
    }
}

/// The `Blake3SM` struct encapsulates the logic of the Blake3 State Machine.
pub struct Blake3SM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Number of available blake3s in the trace.
    pub num_available_blake3s: usize,

    range_id: usize,

    table_id: usize,
}

impl<F: PrimeField64> Blake3SM<F> {
    /// Creates a new Blake3 State Machine instance.
    ///
    /// # Returns
    /// A new `Blake3SM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Compute some useful values
        let num_available_blake3s = Blake3fTrace::<Blake3fTraceRow<F>>::NUM_ROWS / CLOCKS;

        let range_id = std.get_range_id(0, (1 << 16) - 1, None).expect("Failed to get range ID");

        let table_id = std
            .get_virtual_table_id(Blake3fTableSM::TABLE_ID)
            .expect("Failed to get Blake3f table ID");

        Arc::new(Self { std, num_available_blake3s, range_id, table_id })
    }

    /// Processes one operation, filling its CLOCKS-row chunk of the trace and
    /// updating the range-check and XOR-table multiplicities.
    ///
    /// # Arguments
    /// * `input` - The operation data to process.
    /// * `trace` - The CLOCKS-row chunk of the trace assigned to this operation.
    /// * `range_checks` - Multiplicities of the 16-bit range checks.
    /// * `xor_checks` - Multiplicities of the Blake3f XOR⊕ROTR table rows.
    #[inline(always)]
    pub fn process_input<R: Blake3fTraceRowOps<F>>(
        &self,
        input: &Blake3Input,
        trace: &mut [R],
        range_checks: &mut [u32],
        xor_checks: &mut [u32],
    ) {
        // Fill the step_addr
        trace[0].set_step_addr(LANE, input.step_main); // STEP_MAIN
        trace[1].set_step_addr(LANE, input.addr_main as u64); // ADDR_OP
        trace[2].set_step_addr(LANE, input.state_addr as u64); // ADDR_STATE
        trace[3].set_step_addr(LANE, input.input_addr as u64); // ADDR_INPUT
        trace[4].set_step_addr(LANE, input.state_addr as u64); // ADDR_IND_0
        trace[5].set_step_addr(LANE, input.input_addr as u64); // ADDR_IND_1

        // View the state and the message block as 16 little-endian u32 words each
        let mut v = [0u32; 16];
        let mut m = [0u32; 16];
        for i in 0..8 {
            v[2 * i] = input.state[i] as u32;
            v[2 * i + 1] = (input.state[i] >> 32) as u32;
            m[2 * i] = input.input[i] as u32;
            m[2 * i + 1] = (input.input[i] >> 32) as u32;
        }

        for (k, row) in trace.iter_mut().enumerate().take(CLOCKS) {
            let s = &SIGMA[k / NUM_G_PER_ROUND];
            let g = k % NUM_G_PER_ROUND;

            row.set_in_use(LANE, true);

            // Message words consumed by this row's G function
            let x = m[s[2 * g]];
            let y = m[s[2 * g + 1]];
            let x_limbs = u32_to_limbs16(x);
            let y_limbs = u32_to_limbs16(y);
            row.set_all_x(&[x_limbs]);
            row.set_all_y(&[y_limbs]);
            for limb in x_limbs {
                range_checks[limb as usize] += 1;
            }
            for limb in y_limbs {
                range_checks[limb as usize] += 1;
            }

            // Compute the G function
            let (ia, ib, ic, id) = G_INDICES[g];
            let (va, vb, vc, vd) = (v[ia], v[ib], v[ic], v[id]);

            let va_p = va.wrapping_add(vb).wrapping_add(x);
            let vd_p = (vd ^ va_p).rotate_right(R1_G);
            let vc_p = vc.wrapping_add(vd_p);
            let z1 = vb ^ vc_p; // vb' before the rotation
            let vb_p = z1.rotate_right(R2_G);
            let va_pp = va_p.wrapping_add(vb_p).wrapping_add(y);
            let vd_pp = (vd_p ^ va_pp).rotate_right(R3_G);
            let vc_pp = vc_p.wrapping_add(vd_pp);
            let z = vb_p ^ vc_pp; // vb'' before the rotation
            let vb_pp = z.rotate_right(R4_G);

            // Inputs: va/vc as 16-bit limbs (range checked), vb/vd as bytes
            let va_limbs = u32_to_limbs16(va);
            let vc_limbs = u32_to_limbs16(vc);
            row.set_all_va(&[va_limbs]);
            row.set_all_vc(&[vc_limbs]);
            for limb in va_limbs {
                range_checks[limb as usize] += 1;
            }
            for limb in vc_limbs {
                range_checks[limb as usize] += 1;
            }

            let vb_bytes = vb.to_le_bytes();
            let vd_bytes = vd.to_le_bytes();
            row.set_all_vb(&[vb_bytes]);
            row.set_all_vd(&[vd_bytes]);

            // Intermediate and output values as bytes
            let va_p_bytes = va_p.to_le_bytes();
            let vd_p_bytes = vd_p.to_le_bytes();
            let vc_p_bytes = vc_p.to_le_bytes();
            let vb_p_bytes = vb_p.to_le_bytes();
            let va_pp_bytes = va_pp.to_le_bytes();
            let vd_pp_bytes = vd_pp.to_le_bytes();
            let vc_pp_bytes = vc_pp.to_le_bytes();
            let z_bytes = z.to_le_bytes();
            row.set_all_va_prime(&[va_p_bytes]);
            row.set_all_vd_prime(&[vd_p_bytes]);
            row.set_all_vc_prime(&[vc_p_bytes]);
            row.set_all_va_prime_prime(&[va_pp_bytes]);
            row.set_all_vd_prime_prime(&[vd_pp_bytes]);
            row.set_all_vc_prime_prime(&[vc_pp_bytes]);
            row.set_all_vb_pp_xor(&[z_bytes]);

            // vb' as the two byte pieces the rot-12 table rows return for each
            // input byte of z1 = vb ^ vc': piece0 = low nibble shifted up,
            // piece1 = high nibble (the PIL reassembles the rotated bytes)
            let z1_bytes = z1.to_le_bytes();
            let mut vb_p_pieces = [[0u8; 2]; 4];
            for (piece, z1_byte) in vb_p_pieces.iter_mut().zip(z1_bytes) {
                piece[0] = (z1_byte & 0x0F) << 4;
                piece[1] = z1_byte >> 4;
            }
            row.set_all_vb_prime_s(&[vb_p_pieces]);

            // Top bit of rotr8(z), i.e. bit 7 of z's byte 0 (the rotl-by-1 carry)
            row.set_vb_pp_t(LANE, (z >> 7) & 1 == 1);

            // XOR table lookups: (vd, va', rot 0), (vb, vc', rot 12), (vd', va'', rot 0)
            // and (vb', vc'', rot 0), per byte
            for i in 0..4 {
                let rows = [
                    Blake3fTableSM::calculate_table_row(vd_bytes[i], va_p_bytes[i], 0),
                    Blake3fTableSM::calculate_table_row(vb_bytes[i], vc_p_bytes[i], 12),
                    Blake3fTableSM::calculate_table_row(vd_p_bytes[i], va_pp_bytes[i], 0),
                    Blake3fTableSM::calculate_table_row(vb_p_bytes[i], vc_pp_bytes[i], 0),
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

        fn u32_to_limbs16(value: u32) -> [u16; 2] {
            [value as u16, (value >> 16) as u16]
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
    pub fn compute_witness<R: Blake3fTraceRowOps<F>>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<Blake3Input>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = Blake3fTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();
        let num_available_blake3s = self.num_available_blake3s;

        // Check that we can fit all the blake3s in the trace
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        if num_inputs > num_available_blake3s {
            panic!(
                "Exceeded available Blake3s inputs: requested {}, but only {} are available.",
                num_inputs, num_available_blake3s
            );
        }
        let num_rows_filled = num_inputs * CLOCKS;

        tracing::debug!(
            "··· Creating Blake3 instance [{} / {} rows filled {:.2}%]",
            num_rows_filled,
            num_rows,
            num_rows_filled as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(BLAKE3_TRACE);

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
                || (vec![0u32; 1 << 16], vec![0u32; BLAKE3F_TABLE_SIZE]),
                |(mut range_checks, mut xor_checks), (index, trace)| {
                    let input_index = inputs_indexes[index];
                    let input = &inputs[input_index.0][input_index.1];
                    self.process_input::<R>(input, trace, &mut range_checks, &mut xor_checks);
                    (range_checks, xor_checks)
                },
            )
            .reduce(
                || (vec![0u32; 1 << 16], vec![0u32; BLAKE3F_TABLE_SIZE]),
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

        timer_stop_and_log_trace!(BLAKE3_TRACE);

        self.std.range_check_ranged(self.range_id, None, &range_checks);

        let zero_rot0_row = Blake3fTableSM::calculate_table_row(0, 0, 0) as usize;
        let zero_rot12_row = Blake3fTableSM::calculate_table_row(0, 0, 12) as usize;
        xor_checks.into_par_iter().enumerate().for_each(|(row, mut value)| {
            if row == zero_rot0_row {
                value += XOR_ROT0_CHECKS_PER_ROW as u32 * num_padding_rows;
            }
            if row == zero_rot12_row {
                value += XOR_ROT12_CHECKS_PER_ROW as u32 * num_padding_rows;
            }
            if value > 0 {
                self.std.inc_virtual_row(self.table_id, row as u32, value);
            }
        });

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}
