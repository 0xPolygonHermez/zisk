use std::marker::PhantomData;
use std::sync::Arc;

use proofman_fields::PrimeField64;

use proofman_common::{AirInstance, FromTrace, GenericTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

use super::{keccakf_constants::*, KeccakfChiTableSM};
use zisk_common::OperationKeccakData;
use zisk_pil::{KeccakfTraceRow, KeccakfTraceRowOps, KeccakfTraceRowPacked, ZISK_AIRGROUP_ID};

use rayon::prelude::*;

/// Per-operation input record assembled from the bus payload.
#[derive(Debug)]
pub struct KeccakfInput {
    pub step_main: u64,
    pub addr_main: u32,
    pub state: [u64; 25],
}

impl KeccakfInput {
    pub fn from(values: &OperationKeccakData<u64>) -> Self {
        Self {
            step_main: values[4],
            addr_main: values[3] as u32,
            state: values[5..30].try_into().unwrap(),
        }
    }
}

/// The `KeccakfSM` struct encapsulates the logic of the Keccakf State Machine.
/// Nothing here depends on the height of the air: the capacity is taken from the trace each call
/// builds, so a taller sibling would need no change.
pub struct KeccakfSM<F: PrimeField64> {
    _phantom: PhantomData<F>,
}

/// Per-instance round data derived from a clean (bit-valued) state:
/// column sums (values in [0,5]) and their parities.
pub type LaneState = [u64; 25];

/// Packed-row bit layout, mirroring the generated `KeccakfTraceRow`
/// declaration: four activation flags, then the group-row's four-bit state
/// cells, then its four-bit parity cells. Everything else is derived from the
/// PIL's `lanes_per_row`, so changing it moves these offsets with it.
const FLAG_BITS: usize = 4;
const CELL_BITS: usize = 4;
const CELLS_PER_WORD: usize = LANE_BITS / CELL_BITS;
const WORDS_PER_LANE: usize = LANE_BITS / CELLS_PER_WORD;
const STATE_WORDS: usize = LANES_PER_ROW * WORDS_PER_LANE;
const STATE_BIT_OFFSET: usize = FLAG_BITS;
const STATE_BIT_LEN: usize = BITS_PER_ROW * CELL_BITS;
const C_WORDS: usize = C_PER_ROW.div_ceil(CELLS_PER_WORD);
const C_BIT_OFFSET: usize = STATE_BIT_OFFSET + STATE_BIT_LEN;
const C_BIT_LEN: usize = C_PER_ROW * CELL_BITS;

/// Spread sixteen bits into the low bit of sixteen consecutive nibbles.
///
/// A packed Keccak trace cell is four bits.  Applying this to the A and B
/// bit-planes separately lets the hot witness path construct sixteen sliced
/// cells with a handful of word operations instead of sixteen scalar setters.
#[inline(always)]
fn spread_16_to_nibbles(mut value: u64) -> u64 {
    value &= 0xffff;
    value = (value | (value << 24)) & 0x0000_00ff_0000_00ff;
    value = (value | (value << 12)) & 0x000f_000f_000f_000f;
    value = (value | (value << 6)) & 0x0303_0303_0303_0303;
    (value | (value << 3)) & 0x1111_1111_1111_1111
}

#[inline(always)]
fn pack_sliced_lanes(a: &[u64], b: &[u64], out: &mut [u64]) {
    debug_assert_eq!(a.len(), b.len());
    debug_assert_eq!(out.len(), a.len() * WORDS_PER_LANE);
    for lane in 0..a.len() {
        for chunk in 0..WORDS_PER_LANE {
            let shift = chunk * 16;
            let a_bits = spread_16_to_nibbles(a[lane] >> shift);
            let b_bits = spread_16_to_nibbles(b[lane] >> shift);
            out[lane * WORDS_PER_LANE + chunk] = a_bits | (b_bits << 3);
        }
    }
}

/// Sliced parity cells of group-row `row`: grid position p = x·64 + z lives at
/// group-row p / C_PER_ROW, column p % C_PER_ROW. Columns past position 320
/// (only reachable when ROWS_PER_STATE does not divide 320) stay zero.
#[inline(always)]
fn c_cells(row: usize, a: &[u64; 5], b: &[u64; 5]) -> [u8; C_PER_ROW] {
    let mut cells = [0u8; C_PER_ROW];
    for (j, cell) in cells.iter_mut().enumerate() {
        let pos = row * C_PER_ROW + j;
        if pos < 320 {
            let (x, z) = (pos / LANE_BITS, pos % LANE_BITS);
            *cell = ((a[x] >> z) & 1) as u8 + SLOT * ((b[x] >> z) & 1) as u8;
        }
    }
    cells
}

/// OR `nbits` bits of `src` (little-endian, 64 per word) into `packed` starting
/// at absolute bit `bit_offset`, leaving every bit outside that range untouched.
#[inline(always)]
fn blit_bits(packed: &mut [u64], bit_offset: usize, src: &[u64], nbits: usize) {
    debug_assert!(nbits <= src.len() * LANE_BITS);
    let mut done = 0;
    while done < nbits {
        let take = (nbits - done).min(LANE_BITS);
        let mask = if take == LANE_BITS { u64::MAX } else { (1u64 << take) - 1 };
        let value = src[done / LANE_BITS] & mask;
        let at = bit_offset + done;
        let (word, shift) = (at / LANE_BITS, at % LANE_BITS);
        packed[word] = (packed[word] & !(mask << shift)) | (value << shift);
        if shift + take > LANE_BITS {
            let spill = shift + take - LANE_BITS;
            let spill_mask = (1u64 << spill) - 1;
            packed[word + 1] = (packed[word + 1] & !spill_mask) | (value >> (take - spill));
        }
        done += take;
    }
}

/// Keccak-specific bulk writer. A state group spans ROWS_PER_STATE rows, with
/// group-row k holding lanes [k·LANES_PER_ROW, (k+1)·LANES_PER_ROW) and parity
/// positions [k·C_PER_ROW, (k+1)·C_PER_ROW); both methods write one such row.
/// The fallback keeps the generic unpacked representation, while the GPU path
/// writes the generated packed row directly, at offsets derived from the row
/// declaration: four flag bits, the row's state cells, then its parity cells.
#[doc(hidden)]
pub trait KeccakfTraceWriter<F: PrimeField64>: KeccakfTraceRowOps<F> {
    fn set_state_lanes(&mut self, row: usize, a: &LaneState, b: &LaneState);
    fn set_c_parities(&mut self, row: usize, a: &[u64; 5], b: &[u64; 5]);
}

impl<F: PrimeField64> KeccakfTraceWriter<F> for KeccakfTraceRow<F> {
    #[inline(always)]
    fn set_state_lanes(&mut self, row: usize, a: &LaneState, b: &LaneState) {
        let mut cells = [0u8; BITS_PER_ROW];
        let first = row * LANES_PER_ROW;
        for lane in 0..LANES_PER_ROW {
            for z in 0..LANE_BITS {
                cells[lane * LANE_BITS + z] =
                    ((a[first + lane] >> z) & 1) as u8 + SLOT * ((b[first + lane] >> z) & 1) as u8;
            }
        }
        self.set_all_state(&cells);
    }

    #[inline(always)]
    fn set_c_parities(&mut self, row: usize, a: &[u64; 5], b: &[u64; 5]) {
        self.set_all_c(&c_cells(row, a, b));
    }
}

impl<F: PrimeField64> KeccakfTraceWriter<F> for KeccakfTraceRowPacked<F> {
    #[inline(always)]
    fn set_state_lanes(&mut self, row: usize, a: &LaneState, b: &LaneState) {
        debug_assert!(self.packed.len() * LANE_BITS >= C_BIT_OFFSET + C_BIT_LEN);
        let mut words = [0u64; STATE_WORDS];
        let first = row * LANES_PER_ROW;
        let lanes = first..first + LANES_PER_ROW;
        pack_sliced_lanes(&a[lanes.clone()], &b[lanes], &mut words);
        blit_bits(&mut self.packed, STATE_BIT_OFFSET, &words, STATE_BIT_LEN);
    }

    #[inline(always)]
    fn set_c_parities(&mut self, row: usize, a: &[u64; 5], b: &[u64; 5]) {
        let mut words = [0u64; C_WORDS];
        if C_PER_ROW % LANE_BITS == 0 {
            // Each group-row covers whole parity lanes, so the same nibble
            // spreader that packs the state packs the parities too.
            let lanes_per_row = C_PER_ROW / LANE_BITS;
            let first = row * lanes_per_row;
            let lanes = first..first + lanes_per_row;
            pack_sliced_lanes(&a[lanes.clone()], &b[lanes], &mut words);
        } else {
            for (j, cell) in c_cells(row, a, b).iter().enumerate() {
                words[j / CELLS_PER_WORD] |= (*cell as u64) << (CELL_BITS * (j % CELLS_PER_WORD));
            }
        }
        blit_bits(&mut self.packed, C_BIT_OFFSET, &words, C_BIT_LEN);
    }
}

struct ThetaColumns {
    sums: [[u8; 64]; 5],
    parities: [u64; 5],
}

impl ThetaColumns {
    #[inline(always)]
    fn from_state(state: &LaneState) -> Self {
        let mut sums = [[0u8; 64]; 5];
        let mut parities = [0u64; 5];
        for x in 0..5 {
            let lanes = [state[x], state[x + 5], state[x + 10], state[x + 15], state[x + 20]];
            parities[x] = lanes.into_iter().reduce(|a, b| a ^ b).unwrap();
            for (z, sum) in sums[x].iter_mut().enumerate() {
                *sum = lanes.iter().map(|lane| ((lane >> z) & 1) as u8).sum();
            }
        }
        Self { sums, parities }
    }
}

/// Compute the θ output as two bit planes (sum in [0,3]), then apply ρπ.
/// The low plane is also the clean mod-2 state consumed by χ.
#[inline(always)]
fn theta_rho_pi(state: &LaneState, parities: &[u64; 5]) -> (LaneState, LaneState) {
    let mut lo = [0u64; 25];
    let mut hi = [0u64; 25];
    for y in 0..5 {
        for x in 0..5 {
            // χ-position (x,y) reads ρπ from source (x+3y,x).
            let sx = (x + 3 * y) % 5;
            let sy = x;
            let a = state[sx + 5 * sy];
            let b = parities[(sx + 4) % 5];
            let c = parities[(sx + 1) % 5].rotate_left(1);
            let rotation = RHO_OFFSETS[sx][sy] as u32;
            lo[x + 5 * y] = (a ^ b ^ c).rotate_left(rotation);
            hi[x + 5 * y] = ((a & b) | (a & c) | (b & c)).rotate_left(rotation);
        }
    }
    (lo, hi)
}

/// Standard lane-wise χ and ι over the clean low θ plane.
#[inline(always)]
fn chi_iota(b: &LaneState, round: usize) -> LaneState {
    let mut next = [0u64; 25];
    for y in 0..5 {
        for x in 0..5 {
            next[x + 5 * y] = b[x + 5 * y] ^ ((!b[(x + 1) % 5 + 5 * y]) & b[(x + 2) % 5 + 5 * y]);
        }
    }
    next[0] ^= RC[round];
    next
}

impl<F: PrimeField64> KeccakfSM<F> {
    /// Creates a new Keccakf State Machine instance.
    ///
    ///
    /// # Returns
    /// A new `KeccakfSM` instance.
    pub fn new() -> Arc<Self> {
        Arc::new(Self { _phantom: PhantomData })
    }

    /// Processes one slot: fills its CLOCKS-row block of the trace with the two
    /// operations' data and accumulates the lookups into the table histograms.
    ///
    /// The GROUP_IN_A/GROUP_IN_B and GROUP_OUT_A/GROUP_OUT_B state-groups hold
    /// the plain input and output bits of each op; the round groups hold the
    /// SLICED states a + 8·b together with the sliced column parities c. When op
    /// B is absent, its half runs Keccak-f of the zero state (its memory and bus
    /// flags stay off).
    #[inline(always)]
    #[allow(clippy::needless_range_loop)]
    fn process_slot<R: KeccakfTraceWriter<F>>(
        &self,
        trace: &mut [R],
        input_a: &KeccakfInput,
        input_b: Option<&KeccakfInput>,
    ) {
        // Fill step and addr of both ops
        trace[0].set_step_addr(input_a.step_main);
        trace[1].set_step_addr(input_a.addr_main as u64);
        if let Some(input_b) = input_b {
            trace[2].set_step_addr(input_b.step_main);
            trace[3].set_step_addr(input_b.addr_main as u64);
        }

        // Fill the activation flags
        for i in 0..CLOCKS {
            trace[i].set_in_use_a(true);
            trace[i].set_in_use_b(input_b.is_some());
        }

        // Keep the clean Keccak states in their native 25-lane representation.
        // The AIR still receives the identical bit-expanded cells below.
        let mut state_a = input_a.state;
        let mut state_b = input_b.map_or([0u64; 25], |b| b.state);

        // Boundary input groups: plain bits
        Self::set_lane_group(trace, GROUP_IN_A, &state_a);
        Self::set_lane_group(trace, GROUP_IN_B, &state_b);

        // Round groups
        let mut ta = [0u8; 5];
        let mut tb = [0u8; 5];
        let mut chi_accs = [0u32; LANE_BITS];
        for r in 0..=ROUNDS {
            // Sliced state-group of round r
            let group = GROUP_ROUND_0 + r * ROWS_PER_STATE;
            for row in 0..ROWS_PER_STATE {
                trace[group + row].set_state_lanes(row, &state_a, &state_b);
            }

            if r == ROUNDS {
                break;
            }

            // θ columns of both instances
            let cols_a = ThetaColumns::from_state(&state_a);
            let cols_b = ThetaColumns::from_state(&state_b);
            let (theta_lo_a, theta_hi_a) = theta_rho_pi(&state_a, &cols_a.parities);
            let (theta_lo_b, theta_hi_b) = theta_rho_pi(&state_b, &cols_b.parities);

            // Committed sliced parities: position p = x·64+z lives at group-row
            // p / C_PER_ROW, column p % C_PER_ROW
            for row in 0..ROWS_PER_STATE {
                trace[group + row].set_c_parities(row, &cols_a.parities, &cols_b.parities);
            }

            // xor5 lookups: MUST mirror the AIR's batching — at each round row,
            // three c-column slots per lookup, where slot j of group-row `row`
            // holds position row·C_PER_ROW + j; tail slots are zero-padded and
            // triples never cross a row boundary
            for row in 0..ROWS_PER_STATE {
                for g in 0..XOR5_GROUPS {
                    let mut sums = [(0u8, 0u8); XOR5_BATCH];
                    for k in 0..XOR5_BATCH {
                        let j = g * XOR5_BATCH + k;
                        let pos = row * C_PER_ROW + j;
                        if j < C_PER_ROW && pos < 320 {
                            let (x, z) = (pos / 64, pos % 64);
                            sums[k] = (cols_a.sums[x][z], cols_b.sums[x][z]);
                        }
                    }
                }
            }

            // χ-row lookups: one per (y, z); only y = 0 rows carry the ι bit
            for y in 0..5 {
                for z in 0..64 {
                    for x in 0..5 {
                        let index = x + 5 * y;
                        ta[x] = (((theta_lo_a[index] >> z) & 1)
                            | (((theta_hi_a[index] >> z) & 1) << 1))
                            as u8;
                        tb[x] = (((theta_lo_b[index] >> z) & 1)
                            | (((theta_hi_b[index] >> z) & 1) << 1))
                            as u8;
                    }
                    let rc = y == 0 && ((RC[r] >> z) & 1) == 1;
                    // The committed accumulator holds the packed lookup INPUT
                    // (base 28), NOT the compact table-row index (base 16)
                    chi_accs[z] = KeccakfChiTableSM::calculate_table_input(&ta, &tb, rc);
                }

                // On narrow layouts the packed χ-inputs of χ-row group y are
                // committed at its anchor row, the group-row holding lane 5y.
                // chi_acc is declared only in keccakf.pil's ROWS_PER_STATE > 1
                // branch; the wide layout (LANES_PER_ROW = 25) feeds the χ
                // lookups from θ-expressions directly and has no such column,
                // so switching to it drops this write and chi_accs with it.
                trace[group + (5 * y) / LANES_PER_ROW].set_all_chi_acc(&chi_accs);
            }

            // Advance both instances one round
            state_a = chi_iota(&theta_lo_a, r);
            state_b = chi_iota(&theta_lo_b, r);
        }

        // Boundary output groups: plain bits of the final states
        Self::set_lane_group(trace, GROUP_OUT_A, &state_a);
        Self::set_lane_group(trace, GROUP_OUT_B, &state_b);
    }

    /// Writes a clean (bit-valued) state into one boundary group.
    #[inline(always)]
    fn set_lane_group<R: KeccakfTraceWriter<F>>(
        trace: &mut [R],
        first_row: usize,
        state: &LaneState,
    ) {
        for row in 0..ROWS_PER_STATE {
            trace[first_row + row].set_state_lanes(row, state, &[0u64; LANES]);
        }
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    /// The air is selected by the `NUM_ROWS` / `AIR_ID` consts of the trace this builds, so one
    /// body serves every height the air is instantiated at.
    pub fn compute_witness<R: KeccakfTraceWriter<F>, const NUM_ROWS: usize, const AIR_ID: usize>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<KeccakfInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = GenericTrace::<R, NUM_ROWS, ZISK_AIRGROUP_ID, AIR_ID>::new_from_vec_zeroes(
            trace_buffer,
        )?;
        let num_rows = trace.num_rows();

        // Check that we can fit all the keccakfs in the trace
        // Capacity of the air this call builds, taken from `NUM_ROWS`: deriving it from a
        // fixed trace alias instead is what breaks the moment the air gains a taller
        // sibling, since the instance would be measured against the short air's capacity.
        let num_available_keccakfs = OPS_PER_SLOT * (NUM_ROWS / CLOCKS);
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        if num_inputs > num_available_keccakfs {
            panic!(
                "Exceeded available Keccakfs inputs: requested {}, but only {} are available.",
                num_inputs, num_available_keccakfs
            );
        }
        let num_slots_needed = num_inputs.div_ceil(OPS_PER_SLOT);
        let num_rows_needed = num_slots_needed * CLOCKS;

        tracing::debug!(
            "··· Creating Keccakf instance [{} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            num_rows_needed as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(KECCAKF_TRACE);

        // Walk the trace itself, `CLOCKS` rows to a slot, and take each slot's operations by index.
        // The shape used to be four vectors -- the flattened inputs, a Vec of trace slices, a Vec of
        // input pairs, and the two zipped into a third -- because a slot also carried a pair of
        // lookup histograms that had to live somewhere. The prover derives those multiplicities from
        // the committed trace now, so the slot owns nothing but its rows and the two ops that fill
        // them, and the trace can be chunked directly.
        //
        // Slots are paired A-first; a trailing odd operation runs with a zero op B.
        let flat_inputs: Vec<&KeccakfInput> = inputs.iter().flatten().collect();
        let slots_per_chunk = num_slots_needed.div_ceil(rayon::current_num_threads()).max(1);

        trace.buffer[..num_rows_needed]
            .par_chunks_mut(CLOCKS * slots_per_chunk)
            .enumerate()
            .for_each(|(chunk, chunk_rows)| {
                for (i, slot_rows) in chunk_rows.chunks_mut(CLOCKS).enumerate() {
                    let op = (chunk * slots_per_chunk + i) * OPS_PER_SLOT;
                    self.process_slot::<R>(
                        slot_rows,
                        flat_inputs[op],
                        flat_inputs.get(op + 1).copied(),
                    );
                }
            });

        // Update the lookup table multiplicities
        timer_stop_and_log_trace!(KECCAKF_TRACE);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_precomp_helpers::{keccak_f_round, keccakf_state_from_linear, KeccakState};

    fn lanes_from_bits(state: &KeccakState) -> LaneState {
        let mut lanes = [0u64; 25];
        for y in 0..5 {
            for x in 0..5 {
                for (z, bit) in state[x][y].iter().enumerate() {
                    lanes[x + 5 * y] |= (*bit as u64) << z;
                }
            }
        }
        lanes
    }

    #[test]
    fn lane_rounds_and_theta_digits_match_bit_reference() {
        let mut seed = 0x9e3779b97f4a7c15u64;
        for _ in 0..8 {
            let mut lanes = [0u64; 25];
            for lane in &mut lanes {
                seed ^= seed << 7;
                seed ^= seed >> 9;
                seed ^= seed << 8;
                *lane = seed;
            }
            let mut reference = keccakf_state_from_linear(&lanes);
            for round in 0..ROUNDS {
                let columns = ThetaColumns::from_state(&lanes);
                let (lo, hi) = theta_rho_pi(&lanes, &columns.parities);
                for y in 0..5 {
                    for x in 0..5 {
                        let sx = (x + 3 * y) % 5;
                        let sy = x;
                        for z in 0..64 {
                            let sz = (z + 64 - RHO_OFFSETS[sx][sy]) % 64;
                            let expected = reference[sx][sy][sz]
                                + ((columns.parities[(sx + 4) % 5] >> sz) & 1) as u8
                                + ((columns.parities[(sx + 1) % 5] >> ((sz + 63) % 64)) & 1) as u8;
                            let index = x + 5 * y;
                            let actual =
                                (((lo[index] >> z) & 1) | (((hi[index] >> z) & 1) << 1)) as u8;
                            assert_eq!(actual, expected);
                        }
                    }
                }

                lanes = chi_iota(&lo, round);
                keccak_f_round(&mut reference, round);
                reference.iter_mut().flatten().flatten().for_each(|bit| *bit %= 2);
                assert_eq!(lanes, lanes_from_bits(&reference));
            }
        }
    }

    #[test]
    fn packed_bulk_writer_matches_generated_setters() {
        let mut seed = 0xd1b5_4a32_d192_ed03u64;
        for _ in 0..8 {
            let mut a = [0u64; LANES];
            let mut b = [0u64; LANES];
            for lane in a.iter_mut().chain(b.iter_mut()) {
                seed ^= seed << 7;
                seed ^= seed >> 9;
                seed ^= seed << 8;
                *lane = seed;
            }
            let parity_a = ThetaColumns::from_state(&a).parities;
            let parity_b = ThetaColumns::from_state(&b).parities;

            let mut expected = KeccakfTraceRowPacked::<Goldilocks>::default();
            let mut actual = KeccakfTraceRowPacked::<Goldilocks>::default();
            for row in [&mut expected, &mut actual] {
                row.set_in_use_a(true);
                row.set_in_use_b(true);
                row.set_step_addr(0x00ab_cdef_1234);
            }

            for row in 0..ROWS_PER_STATE {
                let mut state_cells = [0u8; BITS_PER_ROW];
                let first = row * LANES_PER_ROW;
                for lane in 0..LANES_PER_ROW {
                    for z in 0..LANE_BITS {
                        state_cells[lane * LANE_BITS + z] = ((a[first + lane] >> z) & 1) as u8
                            + SLOT * ((b[first + lane] >> z) & 1) as u8;
                    }
                }
                expected.set_all_state(&state_cells);
                expected.set_all_c(&c_cells(row, &parity_a, &parity_b));
                actual.set_state_lanes(row, &a, &b);
                actual.set_c_parities(row, &parity_a, &parity_b);

                assert_eq!(actual.packed, expected.packed, "group-row {row}");
            }
        }
    }
}
