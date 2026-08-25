use std::sync::Arc;

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;

use proofman_common::{AirInstance, FromTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

use zisk_common::OperationKeccakData;
use zisk_pil::{KeccakfTrace, KeccakfTraceRow, KeccakfTraceRowOps, KeccakfTraceRowPacked};
use super::{keccakf_constants::*, KeccakfChiTableSM, KeccakfXor5TableSM};

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
pub struct KeccakfSM<F: PrimeField64> {
    /// Number of available keccakfs in the trace.
    pub num_available_keccakfs: usize,

    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,

    /// The virtual table IDs for the χ-row S-box and xor5 tables
    chi_table_id: usize,
    xor5_table_id: usize,
}

/// Per-instance round data derived from a clean (bit-valued) state:
/// column sums (values in [0,5]) and their parities.
type LaneState = [u64; 25];

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
fn pack_sliced_lanes<const LANES: usize>(a: &[u64; LANES], b: &[u64; LANES], out: &mut [u64]) {
    debug_assert_eq!(out.len(), LANES * 4);
    for lane in 0..LANES {
        for chunk in 0..4 {
            let shift = chunk * 16;
            let a_bits = spread_16_to_nibbles(a[lane] >> shift);
            let b_bits = spread_16_to_nibbles(b[lane] >> shift);
            out[lane * 4 + chunk] = a_bits | (b_bits << 3);
        }
    }
}

/// Keccak-specific bulk writer.  The fallback retains the generic unpacked
/// representation, while the reached GPU path writes the generated packed row
/// directly.  The offsets below are derived from the generated row declaration:
/// four flag bits, 1600 four-bit state cells, 320 four-bit parity cells, then
/// the 40-bit step/address field.
trait KeccakfTraceWriter<F: PrimeField64>: KeccakfTraceRowOps<F> {
    fn set_state_lanes(&mut self, a: &LaneState, b: &LaneState);
    fn set_c_parities(&mut self, a: &[u64; 5], b: &[u64; 5]);
}

impl<F: PrimeField64> KeccakfTraceWriter<F> for KeccakfTraceRow<F> {
    #[inline(always)]
    fn set_state_lanes(&mut self, a: &LaneState, b: &LaneState) {
        let mut cells = [0u8; WIDTH];
        for lane in 0..LANES {
            for z in 0..LANE_BITS {
                cells[lane * LANE_BITS + z] =
                    ((a[lane] >> z) & 1) as u8 + SLOT * ((b[lane] >> z) & 1) as u8;
            }
        }
        self.set_all_state(&cells);
    }

    #[inline(always)]
    fn set_c_parities(&mut self, a: &[u64; 5], b: &[u64; 5]) {
        let mut cells = [0u8; 320];
        for x in 0..5 {
            for z in 0..LANE_BITS {
                cells[x * LANE_BITS + z] =
                    ((a[x] >> z) & 1) as u8 + SLOT * ((b[x] >> z) & 1) as u8;
            }
        }
        self.set_all_c(&cells);
    }
}

impl<F: PrimeField64> KeccakfTraceWriter<F> for KeccakfTraceRowPacked<F> {
    #[inline(always)]
    fn set_state_lanes(&mut self, a: &LaneState, b: &LaneState) {
        debug_assert_eq!(self.packed.len(), 121);
        let mut words = [0u64; 100];
        pack_sliced_lanes(a, b, &mut words);

        // State starts at bit four. Preserve the activation flags below it and
        // the parity field above the four-bit spill in word 100.
        self.packed[0] = (self.packed[0] & 0xf) | (words[0] << 4);
        for i in 1..100 {
            self.packed[i] = (words[i - 1] >> 60) | (words[i] << 4);
        }
        self.packed[100] = (self.packed[100] & !0xf) | (words[99] >> 60);
    }

    #[inline(always)]
    fn set_c_parities(&mut self, a: &[u64; 5], b: &[u64; 5]) {
        debug_assert_eq!(self.packed.len(), 121);
        let mut words = [0u64; 20];
        pack_sliced_lanes(a, b, &mut words);

        // Parities start at word 100, bit four. Preserve the state spill below
        // them and the step/address field above their spill in word 120.
        self.packed[100] = (self.packed[100] & 0xf) | (words[0] << 4);
        for i in 1..20 {
            self.packed[100 + i] = (words[i - 1] >> 60) | (words[i] << 4);
        }
        self.packed[120] = (self.packed[120] & !0xf) | (words[19] >> 60);
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
            for z in 0..64 {
                sums[x][z] = lanes.iter().map(|lane| ((lane >> z) & 1) as u8).sum();
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
    /// # Arguments
    /// * `std` - An `Arc`-wrapped reference to the PIL2 standard library.
    ///
    /// # Returns
    /// A new `KeccakfSM` instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Compute some useful values
        let num_available_keccakfs = OPS_PER_SLOT * (KeccakfTrace::<()>::NUM_ROWS / CLOCKS);

        // Get the table IDs
        let chi_table_id = std
            .get_virtual_table_id(KeccakfChiTableSM::TABLE_ID)
            .expect("Failed to get Keccakf χ table ID");
        let xor5_table_id = std
            .get_virtual_table_id(KeccakfXor5TableSM::TABLE_ID)
            .expect("Failed to get Keccakf xor5 table ID");

        Arc::new(Self { num_available_keccakfs, std, chi_table_id, xor5_table_id })
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
        chi_hist: &mut [u32],
        xor5_hist: &mut [u32],
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
        // let mut chi_accs = [0u32; LANE_BITS];
        for r in 0..=ROUNDS {
            // Sliced state-group of round r
            let group = GROUP_ROUND_0 + r * ROWS_PER_STATE;
            debug_assert_eq!(ROWS_PER_STATE, 1);
            trace[group].set_state_lanes(&state_a, &state_b);

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
            trace[group].set_c_parities(&cols_a.parities, &cols_b.parities);

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
                    xor5_hist[KeccakfXor5TableSM::calculate_table_row(&sums) as usize] += 1;
                }
            }

            // χ-row lookups: one per (y, z); only y = 0 rows carry the ι bit
            for y in 0..5 {
                for z in 0..64 {
                    for x in 0..5 {
                        let index = x + 5 * y;
                        ta[x] = (((theta_lo_a[index] >> z) & 1)
                            | (((theta_hi_a[index] >> z) & 1) << 1)) as u8;
                        tb[x] = (((theta_lo_b[index] >> z) & 1)
                            | (((theta_hi_b[index] >> z) & 1) << 1)) as u8;
                    }
                    let rc = y == 0 && ((RC[r] >> z) & 1) == 1;
                    let chi_row = KeccakfChiTableSM::calculate_table_row(&ta, &tb, rc);
                    chi_hist[chi_row as usize] += 1;

                    // The committed accumulator holds the packed lookup INPUT
                    // (base 28), NOT the compact table-row index (base 16)
                    // chi_accs[z] = KeccakfChiTableSM::calculate_table_input(&ta, &tb, rc);
                }

                // On narrow layouts the packed χ-inputs of χ-row group y are
                // committed at its anchor row, the group-row holding lane 5y.
                // NOTE: chi_acc only exists for lanes_per_row < 25; comment out
                //       when instantiating the wide layout.
                // trace[group + (5 * y) / LANES_PER_ROW].set_all_chi_acc(&chi_accs);
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
        debug_assert_eq!(ROWS_PER_STATE, 1);
        trace[first_row].set_state_lanes(state, &[0u64; LANES]);
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness<R: KeccakfTraceWriter<F>>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<KeccakfInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = KeccakfTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = trace.num_rows();

        // Check that we can fit all the keccakfs in the trace
        let num_available_keccakfs = self.num_available_keccakfs;
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

        // Pair the inputs into slots (A-first; a trailing odd op runs with a zero op B)
        let flat_inputs: Vec<&KeccakfInput> = inputs.iter().flatten().collect();
        let mut trace_rows = &mut trace.buffer[..];
        let mut par_traces = Vec::with_capacity(num_slots_needed);
        let mut slot_inputs = Vec::with_capacity(num_slots_needed);
        for pair in flat_inputs.chunks(OPS_PER_SLOT) {
            let (head, tail) = trace_rows.split_at_mut(CLOCKS);
            par_traces.push(head);
            slot_inputs.push((pair[0], pair.get(1).copied()));
            trace_rows = tail;
        }

        // One histogram pair per worker thread. Do NOT use `fold`/`reduce` here:
        // rayon allocates an accumulator per split leaf, and at CHI_TABLE_SIZE =
        // 2^21 each leaf costs an 8 MiB zeroed Vec plus an 8 MiB merge — measured
        // 4.2 s versus 150 ms for this version.
        let mut slots: Vec<_> = par_traces.into_iter().zip(slot_inputs).collect();
        let chunk_size = num_slots_needed.div_ceil(rayon::current_num_threads()).max(1);

        let new_hists =
            || (vec![0u32; CHI_TABLE_SIZE as usize], vec![0u32; XOR5_TABLE_SIZE as usize]);
        let (chi_hist, xor5_hist): (Vec<u32>, Vec<u32>) = slots
            .par_chunks_mut(chunk_size)
            .map(|chunk| {
                let (mut chi, mut xor5) = new_hists();
                for (trace, (input_a, input_b)) in chunk.iter_mut() {
                    self.process_slot::<R>(trace, input_a, *input_b, &mut chi, &mut xor5);
                }
                (chi, xor5)
            })
            .reduce_with(|(mut chi_a, mut xor5_a), (chi_b, xor5_b)| {
                chi_a.iter_mut().zip(chi_b.iter()).for_each(|(a, b)| *a += b);
                xor5_a.iter_mut().zip(xor5_b.iter()).for_each(|(a, b)| *a += b);
                (chi_a, xor5_a)
            })
            .unwrap_or_else(new_hists);

        // Update the lookup table multiplicities
        chi_hist.into_par_iter().enumerate().for_each(|(row, value)| {
            if value > 0 {
                self.std.inc_virtual_row(self.chi_table_id, row as u32, value);
            }
        });
        xor5_hist.into_par_iter().enumerate().for_each(|(row, value)| {
            if value > 0 {
                self.std.inc_virtual_row(self.xor5_table_id, row as u32, value);
            }
        });
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
                for z in 0..64 {
                    lanes[x + 5 * y] |= (state[x][y][z] as u64) << z;
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
                            let actual = (((lo[index] >> z) & 1) | (((hi[index] >> z) & 1) << 1)) as u8;
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

            let mut state_cells = [0u8; WIDTH];
            for lane in 0..LANES {
                for z in 0..LANE_BITS {
                    state_cells[lane * LANE_BITS + z] =
                        ((a[lane] >> z) & 1) as u8 + SLOT * ((b[lane] >> z) & 1) as u8;
                }
            }
            let mut c_cells = [0u8; 320];
            for x in 0..5 {
                for z in 0..LANE_BITS {
                    c_cells[x * LANE_BITS + z] =
                        ((parity_a[x] >> z) & 1) as u8 + SLOT * ((parity_b[x] >> z) & 1) as u8;
                }
            }
            expected.set_all_state(&state_cells);
            expected.set_all_c(&c_cells);
            actual.set_state_lanes(&a, &b);
            actual.set_c_parities(&parity_a, &parity_b);

            assert_eq!(actual.packed, expected.packed);
        }
    }
}
