use core::panic;
use std::sync::Arc;

use proofman_fields::PrimeField64;
use rayon::prelude::*;

use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, GenericTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::OperationBlake3Data;
use zisk_pil::{Blake3fTraceRowOps, ZISK_AIRGROUP_ID};

use super::blake3_constants::{
    BLAKE3F_TABLE_SIZE, CLOCKS, LANES, NUM_G_PER_ROUND, R1_G, R2_G, R3_G, R4_G, SIGMA,
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

/// Writes a 4-byte little-endian value into `lane` of a byte-array column.
macro_rules! set_lane_bytes {
    ($row:expr, $setter:ident, $lane:expr, $bytes:expr) => {
        for (i, byte) in $bytes.into_iter().enumerate() {
            $row.$setter($lane, i, byte);
        }
    };
}

/// Compile-time check that LANES agrees with the lane dimension of the generated trace.
const _: () = {
    #[allow(dead_code)]
    fn assert_lanes<F: PrimeField64, R: Blake3fTraceRowOps<F>>(row: &R) {
        let _: [bool; LANES] = row.get_all_in_use();
    }
};

/// Splits a u32 into its two little-endian 16-bit limbs.
#[inline(always)]
fn u32_to_limbs16(value: u32) -> [u16; 2] {
    [value as u16, (value >> 16) as u16]
}

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

        let range_id = std.get_range_id(0, (1 << 16) - 1, None).expect("Failed to get range ID");

        let table_id = std
            .get_virtual_table_id(Blake3fTableSM::TABLE_ID)
            .expect("Failed to get Blake3f table ID");

        Arc::new(Self { std, range_id, table_id })
    }

    /// Processes one operation, filling one lane of its CLOCKS-row cycle and
    /// updating the range-check and XOR-table multiplicities.
    ///
    /// # Arguments
    /// * `input` - The operation data to process.
    /// * `lane` - The lane of the cycle this operation is placed in.
    /// * `trace` - The CLOCKS-row cycle shared by the LANES operations packed side by side.
    /// * `range_checks` - Multiplicities of the 16-bit range checks.
    /// * `xor_checks` - Multiplicities of the Blake3f XOR⊕ROTR table rows.
    #[inline(always)]
    pub fn process_input<R: Blake3fTraceRowOps<F>>(
        &self,
        input: &Blake3Input,
        lane: usize,
        trace: &mut [R],
        range_checks: &mut [u32],
        xor_checks: &mut [u32],
    ) {
        // Fill the step_addr
        trace[0].set_step_addr(lane, input.step_main); // STEP_MAIN
        trace[1].set_step_addr(lane, input.addr_main as u64); // ADDR_OP
        trace[2].set_step_addr(lane, input.state_addr as u64); // ADDR_STATE
        trace[3].set_step_addr(lane, input.input_addr as u64); // ADDR_INPUT
        trace[4].set_step_addr(lane, input.state_addr as u64); // ADDR_IND_0
        trace[5].set_step_addr(lane, input.input_addr as u64); // ADDR_IND_1

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

            row.set_in_use(lane, true);

            // Message words consumed by this row's G function
            let x = m[s[2 * g]];
            let y = m[s[2 * g + 1]];
            for (i, limb) in u32_to_limbs16(x).into_iter().enumerate() {
                row.set_x(lane, i, limb);
                range_checks[limb as usize] += 1;
            }
            for (i, limb) in u32_to_limbs16(y).into_iter().enumerate() {
                row.set_y(lane, i, limb);
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
            for (i, limb) in u32_to_limbs16(va).into_iter().enumerate() {
                row.set_va(lane, i, limb);
                range_checks[limb as usize] += 1;
            }
            for (i, limb) in u32_to_limbs16(vc).into_iter().enumerate() {
                row.set_vc(lane, i, limb);
                range_checks[limb as usize] += 1;
            }

            let vb_bytes = vb.to_le_bytes();
            let vd_bytes = vd.to_le_bytes();
            set_lane_bytes!(row, set_vb, lane, vb_bytes);
            set_lane_bytes!(row, set_vd, lane, vd_bytes);

            // Intermediate and output values as bytes
            let va_p_bytes = va_p.to_le_bytes();
            let vd_p_bytes = vd_p.to_le_bytes();
            let vc_p_bytes = vc_p.to_le_bytes();
            let vb_p_bytes = vb_p.to_le_bytes();
            let va_pp_bytes = va_pp.to_le_bytes();
            let vd_pp_bytes = vd_pp.to_le_bytes();
            let vc_pp_bytes = vc_pp.to_le_bytes();
            let z_bytes = z.to_le_bytes();
            set_lane_bytes!(row, set_va_prime, lane, va_p_bytes);
            set_lane_bytes!(row, set_vd_prime, lane, vd_p_bytes);
            set_lane_bytes!(row, set_vc_prime, lane, vc_p_bytes);
            set_lane_bytes!(row, set_va_prime_prime, lane, va_pp_bytes);
            set_lane_bytes!(row, set_vd_prime_prime, lane, vd_pp_bytes);
            set_lane_bytes!(row, set_vc_prime_prime, lane, vc_pp_bytes);
            set_lane_bytes!(row, set_vb_pp_xor, lane, z_bytes);

            // vb' as the two byte pieces the rot-12 table rows return for each
            // input byte of z1 = vb ^ vc': piece0 = low nibble shifted up,
            // piece1 = high nibble (the PIL reassembles the rotated bytes)
            for (i, z1_byte) in z1.to_le_bytes().into_iter().enumerate() {
                row.set_vb_prime_s(lane, i, 0, (z1_byte & 0x0F) << 4);
                row.set_vb_prime_s(lane, i, 1, z1_byte >> 4);
            }

            // Top bit of rotr8(z), i.e. bit 7 of z's byte 0 (the rotl-by-1 carry)
            row.set_vb_pp_t(lane, (z >> 7) & 1 == 1);

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
    pub fn compute_witness<R: Blake3fTraceRowOps<F>, const NUM_ROWS: usize, const AIR_ID: usize>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<Blake3Input>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = GenericTrace::<R, NUM_ROWS, ZISK_AIRGROUP_ID, AIR_ID>::new_from_vec_zeroes(
            trace_buffer,
        )?;
        let num_rows = trace.num_rows();
        // Capacity of the air this call builds, taken from `NUM_ROWS`: deriving it from a fixed
        // trace alias instead is what breaks the moment the air gains a taller sibling, since the
        // instance would be measured against the short air's capacity.
        let num_available_blake3s = NUM_ROWS / CLOCKS * LANES;

        // Check that we can fit all the blake3s in the trace
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        if num_inputs > num_available_blake3s {
            panic!(
                "Exceeded available Blake3s inputs: requested {}, but only {} are available.",
                num_inputs, num_available_blake3s
            );
        }
        // Each CLOCKS-row cycle hosts up to LANES operations side by side
        let num_cycles = num_inputs.div_ceil(LANES);
        let num_rows_filled = num_cycles * CLOCKS;

        tracing::debug!(
            "··· Creating Blake3 instance [{} / {} rows filled {:.2}%]",
            num_rows_filled,
            num_rows,
            num_rows_filled as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(BLAKE3_TRACE);

        // Split trace into per-cycle chunks for parallel processing
        let flat_inputs: Vec<&Blake3Input> = inputs.iter().flatten().collect();
        let mut trace_rows = trace.buffer.as_mut_slice();
        let mut par_traces = Vec::with_capacity(num_cycles);
        for _ in 0..num_cycles {
            let (head, tail) = trace_rows.split_at_mut(CLOCKS);
            par_traces.push(head);
            trace_rows = tail;
        }

        // Fill the trace, collecting the range-check and XOR-table multiplicities
        let (mut range_checks, xor_checks) = par_traces
            .into_par_iter()
            .enumerate()
            .fold(
                || (vec![0u32; 1 << 16], vec![0u32; BLAKE3F_TABLE_SIZE]),
                |(mut range_checks, mut xor_checks), (cycle, trace)| {
                    // Lanes must be filled in order: the last cycle may leave the
                    // trailing lanes empty
                    let inputs = &flat_inputs[cycle * LANES..];
                    for (lane, input) in inputs.iter().take(LANES).enumerate() {
                        self.process_input::<R>(
                            input,
                            lane,
                            trace,
                            &mut range_checks,
                            &mut xor_checks,
                        );
                    }
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

        // The range checks and XOR lookups are per lane and unconditional, so every
        // lane-row left empty (a padding row, or a trailing lane of the last cycle)
        // contributes them over zeros
        let num_empty_lane_rows = (num_rows * LANES - num_inputs * CLOCKS) as u32;
        range_checks[0] += RANGE_CHECKED_LIMBS_PER_ROW as u32 * num_empty_lane_rows;

        timer_stop_and_log_trace!(BLAKE3_TRACE);

        self.std.range_check_ranged(self.range_id, None, &range_checks);

        let zero_rot0_row = Blake3fTableSM::calculate_table_row(0, 0, 0) as usize;
        let zero_rot12_row = Blake3fTableSM::calculate_table_row(0, 0, 12) as usize;
        xor_checks.into_par_iter().enumerate().for_each(|(row, mut value)| {
            if row == zero_rot0_row {
                value += XOR_ROT0_CHECKS_PER_ROW as u32 * num_empty_lane_rows;
            }
            if row == zero_rot12_row {
                value += XOR_ROT12_CHECKS_PER_ROW as u32 * num_empty_lane_rows;
            }
            if value > 0 {
                self.std.inc_virtual_row(self.table_id, row as u32, value);
            }
        });

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}
