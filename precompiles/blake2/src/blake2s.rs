use std::sync::Arc;

use proofman_fields::PrimeField64;
use rayon::prelude::*;

use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::OperationBlake2Data;
use zisk_pil::{Blake2srTrace, Blake2srTraceRow, Blake2srTraceRowOps};

use super::blake2_constants::{BLAKE2BR_TABLE_SIZE, CLOCKS, SIGMA};
use super::blake2_table::Blake2brTableSM;

/// State indices (a, b, c, d) mixed by the G function at each clock:
/// clocks 0-3 perform the column mixing, clocks 4-7 the diagonal mixing.
/// Identical to Blake2b — only the word width differs.
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

/// Number of 16-bit range-checked limbs per row: x[2], y[2], va[2], vc[2].
/// Half of Blake2b's, since a 32-bit word is two limbs rather than four.
const RANGE_CHECKED_LIMBS_PER_ROW: usize = 8;

/// Number of unconditional XOR table lookups per row: vd', vb_p_rot16,
/// vd'' and vb_pp_rot24, four bytes each. Half of Blake2b's 32.
const XOR_CHECKS_PER_ROW: usize = 16;

/// BLAKE2s rotation constants.
const R1_G: u32 = 16;
const R2_G: u32 = 12;
const R3_G: u32 = 8;
const R4_G: u32 = 7;

/// Per-operation input record assembled from the bus payload.
///
/// Each 32-bit word arrives in its own 64-bit slot with a zero high half, which
/// is what the AIR's `value: [mem_lo, 0]` memory argument enforces.
#[derive(Debug)]
pub struct Blake2sInput {
    pub addr_main: u32,
    pub step_main: u64,
    pub index: u64,
    pub state_addr: u32,
    pub input_addr: u32,
    pub state: [u32; 16],
    pub input: [u32; 16],
}

impl Blake2sInput {
    pub fn from(values: &OperationBlake2Data<u64>) -> Self {
        let mut state = [0u32; 16];
        let mut input = [0u32; 16];
        for i in 0..16 {
            let s = values[8 + i];
            let m = values[24 + i];
            // Not debug_assert!: release builds would compile the check out and
            // `as u32` would truncate silently, leaving an unprovable trace that
            // only fails later in the global constraints.
            assert_eq!(s >> 32, 0, "blake2s state word {i} has a non-zero high half");
            assert_eq!(m >> 32, 0, "blake2s input word {i} has a non-zero high half");
            state[i] = s as u32;
            input[i] = m as u32;
        }
        Self {
            addr_main: values[3] as u32,
            step_main: values[4],
            index: values[5],
            state_addr: values[6] as u32,
            input_addr: values[7] as u32,
            state,
            input,
        }
    }
}

/// The `Blake2sSM` struct encapsulates the logic of the Blake2s State Machine.
pub struct Blake2sSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Number of available blake2s rounds in the trace.
    pub num_available_blake2s: usize,

    /// 16-bit range for the x/y/va/vc limbs.
    range_id: usize,

    /// 4-bit range for the `>>> 12` shift-and-carry.
    range_id_nibble: usize,

    table_id: usize,
}

impl<F: PrimeField64> Blake2sSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let num_non_usable_rows = Blake2srTrace::<Blake2srTraceRow<F>>::NUM_ROWS % CLOCKS;
        let num_available_blake2s = Blake2srTrace::<Blake2srTraceRow<F>>::NUM_ROWS / CLOCKS
            - (num_non_usable_rows != 0) as usize;

        let range_id = std.get_range_id(0, (1 << 16) - 1, None).expect("Failed to get range ID");
        let range_id_nibble = std.get_range_id(0, 15, None).expect("Failed to get 4-bit range ID");

        // The XOR table is byte-wise and therefore shared with Blake2b verbatim.
        let table_id = std
            .get_virtual_table_id(Blake2brTableSM::TABLE_ID)
            .expect("Failed to get Blake2br table ID");

        Arc::new(Self { std, num_available_blake2s, range_id, range_id_nibble, table_id })
    }

    /// Processes one operation, filling its CLOCKS-row chunk of the trace and
    /// updating the range-check and XOR-table multiplicities.
    #[inline(always)]
    pub fn process_input<R: Blake2srTraceRowOps<F>>(
        &self,
        input: &Blake2sInput,
        trace: &mut [R],
        range_checks: &mut [u32],
        nibble_checks: &mut [u32],
        xor_checks: &mut [u32],
    ) {
        // Reject out-of-range indices rather than reducing them. The AIR encodes
        // round_idx as a one-hot over SIGMA_LENGTH selectors, so it can only ever
        // represent 0..9, and the param port equates it to the index the guest
        // wrote to memory (`blake2sr.pil`, "Param port"). Reducing here would
        // still leave that memory argument unsatisfiable, turning a clear panic
        // into an opaque constraint failure. Callers reduce before the syscall,
        // exactly as `zisklib::blake2s_compress` and `blake2b_compress` do.
        let idx_usize = input.index as usize;
        assert!(
            idx_usize < SIGMA.len(),
            "blake2s round index {idx_usize} exceeds SIGMA ({}); reduce mod {} before the syscall",
            SIGMA.len(),
            SIGMA.len()
        );
        let s = &SIGMA[idx_usize];

        // Fill the step_addr column: the AIR reads these at fixed clock offsets.
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

            // Memory-ordered message words bound by the x/y memory ports
            let x_limbs = u32_to_limbs16(input.input[2 * k]);
            let y_limbs = u32_to_limbs16(input.input[2 * k + 1]);
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
            row.set_xs(xs);
            row.set_ys(ys);

            // ── The G function ──
            let (ia, ib, ic, id) = G_INDICES[k];
            let (va, vb, vc, vd) = (v[ia], v[ib], v[ic], v[id]);

            let va_p = va.wrapping_add(vb).wrapping_add(xs);
            let vd_p = (vd ^ va_p).rotate_right(R1_G);
            let vc_p = vc.wrapping_add(vd_p);

            // >>> 12 = (<<< 16) <<< 4. The AIR's XOR lookup emits the <<< 16
            // half directly into shifted byte columns; the residual <<< 4 is a
            // doubling-style carry, so the intermediate is materialised here.
            let z_p = (vb ^ vc_p).rotate_left(16);
            let vb_p_t = (z_p >> 28) as u8; // top 4 bits
            let vb_p = z_p.rotate_left(4);
            debug_assert_eq!(vb_p, (vb ^ vc_p).rotate_right(R2_G));

            let va_pp = va_p.wrapping_add(vb_p).wrapping_add(ys);
            let vd_pp = (vd_p ^ va_pp).rotate_right(R3_G);
            let vc_pp = vc_p.wrapping_add(vd_pp);

            // >>> 7 = (<<< 24) <<< 1, the same shape Blake2b uses for >>> 63.
            let z_pp = (vb_p ^ vc_pp).rotate_left(24);
            let vb_pp_t = (z_pp >> 31) & 1 == 1;
            let vb_pp = z_pp.rotate_left(1);
            debug_assert_eq!(vb_pp, (vb_p ^ vc_pp).rotate_right(R4_G));

            // ── Inputs: va/vc as 16-bit limbs (range checked), vb/vd as bytes ──
            let va_limbs = u32_to_limbs16(va);
            let vc_limbs = u32_to_limbs16(vc);
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

            // ── Intermediates and outputs as bytes ──
            let va_p_bytes = va_p.to_le_bytes();
            let vd_p_bytes = vd_p.to_le_bytes();
            let vc_p_bytes = vc_p.to_le_bytes();
            let z_p_bytes = z_p.to_le_bytes();
            let vb_p_bytes = vb_p.to_le_bytes();
            let va_pp_bytes = va_pp.to_le_bytes();
            let vd_pp_bytes = vd_pp.to_le_bytes();
            let vc_pp_bytes = vc_pp.to_le_bytes();
            let z_pp_bytes = z_pp.to_le_bytes();

            row.set_all_va_prime(&va_p_bytes);
            row.set_all_vd_prime(&vd_p_bytes);
            row.set_all_vc_prime(&vc_p_bytes);
            row.set_all_vb_p_rot16(&z_p_bytes);
            row.set_vb_p_t(vb_p_t);
            row.set_all_vb_prime(&vb_p_bytes);
            row.set_all_va_prime_prime(&va_pp_bytes);
            row.set_all_vd_prime_prime(&vd_pp_bytes);
            row.set_all_vc_prime_prime(&vc_pp_bytes);
            row.set_all_vb_pp_rot24(&z_pp_bytes);
            row.set_vb_pp_t(vb_pp_t);

            nibble_checks[vb_p_t as usize] += 1;

            // ── XOR table lookups: (vd, va'), (vb, vc'), (vd', va''), (vb', vc'') ──
            // Four per byte, four bytes: 16 per row against Blake2b's 32.
            for i in 0..4 {
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

        fn u32_to_limbs16(value: u32) -> [u16; 2] {
            [value as u16, (value >> 16) as u16]
        }
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    pub fn compute_witness<R: Blake2srTraceRowOps<F>>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<Blake2sInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = Blake2srTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();
        let num_available_blake2s = self.num_available_blake2s;

        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        if num_inputs > num_available_blake2s {
            panic!(
                "Exceeded available Blake2s inputs: requested {}, but only {} are available.",
                num_inputs, num_available_blake2s
            );
        }
        let num_rows_filled = num_inputs * CLOCKS;

        tracing::debug!(
            "··· Creating Blake2s instance [{} / {} rows filled {:.2}%]",
            num_rows_filled,
            num_rows,
            num_rows_filled as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(BLAKE2S_TRACE);

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

        // Fill the trace, collecting range-check, nibble and XOR multiplicities
        let (mut range_checks, mut nibble_checks, xor_checks) = par_traces
            .into_par_iter()
            .enumerate()
            .fold(
                || (vec![0u32; 1 << 16], vec![0u32; 16], vec![0u32; BLAKE2BR_TABLE_SIZE]),
                |(mut range_checks, mut nibble_checks, mut xor_checks), (index, trace)| {
                    let input_index = inputs_indexes[index];
                    let input = &inputs[input_index.0][input_index.1];
                    self.process_input::<R>(
                        input,
                        trace,
                        &mut range_checks,
                        &mut nibble_checks,
                        &mut xor_checks,
                    );
                    (range_checks, nibble_checks, xor_checks)
                },
            )
            .reduce(
                || (vec![0u32; 1 << 16], vec![0u32; 16], vec![0u32; BLAKE2BR_TABLE_SIZE]),
                |(mut range_acc, mut nib_acc, mut xor_acc), (range, nib, xor)| {
                    for (acc, val) in range_acc.iter_mut().zip(range) {
                        *acc += val;
                    }
                    for (acc, val) in nib_acc.iter_mut().zip(nib) {
                        *acc += val;
                    }
                    for (acc, val) in xor_acc.iter_mut().zip(xor) {
                        *acc += val;
                    }
                    (range_acc, nib_acc, xor_acc)
                },
            );

        // Padding rows are all-zero: in_use is off, so the only bus contributions
        // are the unconditional range checks and XOR table lookups over zeros
        trace.buffer[num_rows_filled..num_rows]
            .par_iter_mut()
            .for_each(|slot| *slot = R::default());

        let num_padding_rows = (num_rows - num_rows_filled) as u32;
        range_checks[0] += RANGE_CHECKED_LIMBS_PER_ROW as u32 * num_padding_rows;
        // A zero row carries vb_p_t = 0, one 4-bit range check each.
        nibble_checks[0] += num_padding_rows;

        timer_stop_and_log_trace!(BLAKE2S_TRACE);

        self.std.range_check_ranged(self.range_id, None, &range_checks);
        self.std.range_check_ranged(self.range_id_nibble, None, &nibble_checks);

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

#[cfg(test)]
mod tests {
    /// The AIR expresses `>>> 12` as `(<<< 16) <<< 4` and `>>> 7` as
    /// `(<<< 24) <<< 1`, absorbing the byte-aligned half into the XOR lookup's
    /// output index. If either identity is wrong the trace and the constraints
    /// disagree, so pin them.
    #[test]
    fn rotation_decompositions_hold() {
        for x in [0u32, 1, 0xFFFF_FFFF, 0x8000_0001, 0x1234_5678, 0xDEAD_BEEF, 0x0F0F_0F0F] {
            assert_eq!(x.rotate_left(16).rotate_left(4), x.rotate_right(12), "x={x:#010x}");
            assert_eq!(x.rotate_left(24).rotate_left(1), x.rotate_right(7), "x={x:#010x}");
        }
    }

    /// The carry the AIR range-checks is the top nibble (resp. top bit) of the
    /// byte-rotated intermediate, and `<<< n` is `2^n · z` with that carry
    /// wrapping around. Pin the arithmetic the constraint encodes.
    #[test]
    fn carry_arithmetic_matches_the_constraint() {
        const P2_32: u64 = 1u64 << 32;
        for x in [0u32, 1, 0xFFFF_FFFF, 0x8000_0001, 0x1234_5678, 0xDEAD_BEEF] {
            // <<< 4 with a 4-bit carry
            let t = (x >> 28) as u64;
            let got = 16 * (x as u64) - P2_32 * t + t;
            assert_eq!(got, x.rotate_left(4) as u64, "rotl4 x={x:#010x}");

            // <<< 1 with a 1-bit carry
            let t = (x >> 31) as u64;
            let got = 2 * (x as u64) - P2_32 * t + t;
            assert_eq!(got, x.rotate_left(1) as u64, "rotl1 x={x:#010x}");
        }
    }

    /// The carries are *forced*, not merely range-checked, and that is what
    /// makes the mid-G `>>> 12` sound: the constraint result is byte-bound to
    /// [0, 2^32), and only the true carry lands inside that window. A prover
    /// substituting any other value produces something out of range, which the
    /// byte columns cannot represent.
    ///
    /// This is the adversarial half of `carry_arithmetic_matches_the_constraint`
    /// -- corrupting either carry is rejected -- kept as a unit test so the
    /// argument cannot silently regress.
    ///
    /// Coverage: exhaustive over the carry dimension, which is the one the
    /// argument turns on. Every high-nibble class `h` (all 16, resp. both
    /// high-bit classes) is crossed with every candidate carry `t`, over
    /// representative low bits. It is *not* exhaustive over all 2^32 values of
    /// x; the AIR argument is universal, this test samples `l`.
    #[test]
    fn only_the_true_carry_is_representable() {
        const P2_32: i128 = 1i128 << 32;
        const RANGE: std::ops::Range<i128> = 0..P2_32;

        // Low-bit patterns: the extremes plus a few shapes that would expose a
        // borrow or carry mishandled at a byte or nibble boundary.
        const LOW_28: [u32; 6] = [0, 1, 0x0FFF_FFFF, 0x0FFF_FFFE, 0x0123_4567, 0x0AAA_AAAA];
        const LOW_31: [u32; 6] = [0, 1, 0x7FFF_FFFF, 0x7FFF_FFFE, 0x1234_5678, 0x5555_5555];

        // >>> 12 = (<<< 16) <<< 4, carry is the top nibble: all 16 classes.
        for h in 0..16u32 {
            for l in LOW_28 {
                let x = (h << 28) | l;
                assert_eq!(x >> 28, h, "constructed x={x:#010x} has the wrong class");
                for t in 0..16i128 {
                    let v = 16 * i128::from(x) - P2_32 * t + t;
                    assert_eq!(
                        RANGE.contains(&v),
                        t == i128::from(h),
                        "rotl4 x={x:#010x} t={t}: only the true carry {h} may be representable"
                    );
                }
            }
        }

        // >>> 7 = (<<< 24) <<< 1, carry is the top bit: both classes.
        for h in 0..2u32 {
            for l in LOW_31 {
                let x = (h << 31) | l;
                assert_eq!(x >> 31, h, "constructed x={x:#010x} has the wrong class");
                for t in 0..2i128 {
                    let v = 2 * i128::from(x) - P2_32 * t + t;
                    assert_eq!(
                        RANGE.contains(&v),
                        t == i128::from(h),
                        "rotl1 x={x:#010x} t={t}: only the true carry {h} may be representable"
                    );
                }
            }
        }
    }

    /// Adversarial: each 32-bit word rides in its own 8-byte slot with a zero
    /// high half, which the AIR's `value: [mem_lo, 0]` memory argument enforces.
    /// A poisoned slot was measured to be rejected only by the *global*
    /// constraints -- the Blake2sr AIR itself verified, having seen the
    /// truncated value -- so reject it here, where the diagnosis is clear.
    #[test]
    #[should_panic(expected = "non-zero high half")]
    fn rejects_state_word_with_high_half() {
        let mut values = [0u64; zisk_common::OPERATION_BUS_BLAKE2_DATA_SIZE];
        values[8] = 1u64 << 32; // first state word, high half set
        let _ = super::Blake2sInput::from(&values);
    }

    /// The carry argument assumes no value can wrap back into range modulo the
    /// Goldilocks prime. Pin the premise on the extremes, which bound the
    /// expression: the widest excursions come from x at 0 or 2^32-1 crossed
    /// with the largest candidate carry, and even those stay far inside the
    /// field, so `p` never rescues an out-of-range witness.
    #[test]
    fn carry_expressions_stay_inside_the_field() {
        const P2_32: i128 = 1i128 << 32;
        const GOLDILOCKS: i128 = (1i128 << 64) - (1i128 << 32) + 1;

        let mut worst = 0i128;
        for x in [0u32, 1, 0xFFFF_FFFF, 0x8000_0001, 0x7FFF_FFFF] {
            for t in 0..16i128 {
                worst = worst.max((16 * i128::from(x) - P2_32 * t + t).abs());
            }
            for t in 0..2i128 {
                worst = worst.max((2 * i128::from(x) - P2_32 * t + t).abs());
            }
        }
        assert!(worst < GOLDILOCKS, "carry expression {worst} can alias mod p");
    }
}
