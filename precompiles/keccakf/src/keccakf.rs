use std::sync::Arc;

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;

use proofman_common::{AirInstance, FromTrace, GenericTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

use zisk_common::OperationKeccakData;
use zisk_pil::{KeccakfTraceRowOps, ZISK_AIRGROUP_ID};

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
/// Nothing here depends on the height of the air: the capacity is taken from the trace each call
/// builds, so a taller sibling would need no change.
pub struct KeccakfSM<F: PrimeField64> {
    /// Number of available keccakfs in the trace.

    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,

    /// The virtual table IDs for the χ-row S-box and xor5 tables
    chi_table_id: usize,
    xor5_table_id: usize,
}

/// A clean Keccak state in its native lane form: lane `x + 5y`, bit `z`.
type LaneState = [u64; LANES];

/// Per-instance round data derived from a clean state: the column sums (values
/// in [0,5]) as three bit-planes, and their parities.
struct ThetaColumns {
    /// sum(x, z) = bit z of `sum_planes[0..3][x]`, weighted 1, 2, 4.
    sum_planes: [[u64; 5]; 3],
    parities: [u64; 5],
}

impl ThetaColumns {
    #[inline(always)]
    fn from_state(state: &LaneState) -> Self {
        let mut sum_planes = [[0u64; 5]; 3];
        let mut parities = [0u64; 5];
        for x in 0..5 {
            let (a, b, c, d, e) =
                (state[x], state[x + 5], state[x + 10], state[x + 15], state[x + 20]);
            // Two full adders over the five bit-planes, then the carries.
            let s1 = a ^ b ^ c;
            let c1 = (a & b) | (a & c) | (b & c);
            let s0 = s1 ^ d ^ e;
            let c2 = (s1 & d) | (s1 & e) | (d & e);
            sum_planes[0][x] = s0;
            sum_planes[1][x] = c1 ^ c2;
            sum_planes[2][x] = c1 & c2;
            parities[x] = s0;
        }
        Self { sum_planes, parities }
    }

    #[inline(always)]
    fn sum_at(&self, x: usize, z: usize) -> u8 {
        (((self.sum_planes[0][x] >> z) & 1)
            + 2 * ((self.sum_planes[1][x] >> z) & 1)
            + 4 * ((self.sum_planes[2][x] >> z) & 1)) as u8
    }
}

/// θ then ρπ, as two bit-planes of the per-position value in [0,3]: `lo + 2·hi`.
/// `lo` is also the clean mod-2 state that χ consumes.
#[inline(always)]
fn theta_rho_pi(state: &LaneState, parities: &[u64; 5]) -> (LaneState, LaneState) {
    let mut lo = [0u64; LANES];
    let mut hi = [0u64; LANES];
    for y in 0..5 {
        for x in 0..5 {
            // χ-position (x,y) reads ρπ from source (x+3y, x).
            let (sx, sy) = ((x + 3 * y) % 5, x);
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

/// Lane-wise χ and ι over the clean low θ plane.
#[inline(always)]
fn chi_iota(b: &LaneState, round: usize) -> LaneState {
    let mut next = [0u64; LANES];
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

        // Get the table IDs
        let chi_table_id = std
            .get_virtual_table_id(KeccakfChiTableSM::TABLE_ID)
            .expect("Failed to get Keccakf χ table ID");
        let xor5_table_id = std
            .get_virtual_table_id(KeccakfXor5TableSM::TABLE_ID)
            .expect("Failed to get Keccakf xor5 table ID");

        Arc::new(Self { std, chi_table_id, xor5_table_id })
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
    fn process_slot<R: KeccakfTraceRowOps<F>>(
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

        // The two clean states stay in their native 25-lane form throughout.
        let mut state_a = input_a.state;
        let mut state_b = input_b.map_or([0u64; LANES], |b| b.state);

        // Boundary input groups: plain bits
        Self::set_state_group(trace, GROUP_IN_A, &state_a, &[0u64; LANES]);
        Self::set_state_group(trace, GROUP_IN_B, &state_b, &[0u64; LANES]);

        // Round groups
        let mut ta = [0u8; 5];
        let mut tb = [0u8; 5];
        let mut chi_accs = [0u32; LANE_BITS];
        for r in 0..=ROUNDS {
            // Sliced state-group of round r
            let group = GROUP_ROUND_0 + r * ROWS_PER_STATE;
            Self::set_state_group(trace, group, &state_a, &state_b);

            if r == ROUNDS {
                break;
            }

            // θ columns of both instances, then θρπ as two bit-planes
            let cols_a = ThetaColumns::from_state(&state_a);
            let cols_b = ThetaColumns::from_state(&state_b);
            let (lo_a, hi_a) = theta_rho_pi(&state_a, &cols_a.parities);
            let (lo_b, hi_b) = theta_rho_pi(&state_b, &cols_b.parities);

            // Committed sliced parities: position p = x·64+z lives at group-row
            // p / C_PER_ROW, column p % C_PER_ROW
            for row in 0..ROWS_PER_STATE {
                let mut c_cells = [0u8; C_PER_ROW];
                for (j, c_cell) in c_cells.iter_mut().enumerate() {
                    let pos = row * C_PER_ROW + j;
                    if pos < 320 {
                        let (x, z) = (pos / 64, pos % 64);
                        *c_cell = (((cols_a.parities[x] >> z) & 1)
                            + SLOT as u64 * ((cols_b.parities[x] >> z) & 1))
                            as u8;
                    }
                }
                trace[group + row].set_all_c(&c_cells);
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
                            sums[k] = (cols_a.sum_at(x, z), cols_b.sum_at(x, z));
                        }
                    }
                    xor5_hist[KeccakfXor5TableSM::calculate_table_row(&sums) as usize] += 1;
                }
            }

            // χ-row lookups: one per (y, z); only y = 0 rows carry the ι bit
            for y in 0..5 {
                for z in 0..64 {
                    for x in 0..5 {
                        let lane = x + 5 * y;
                        ta[x] = (((lo_a[lane] >> z) & 1) + 2 * ((hi_a[lane] >> z) & 1)) as u8;
                        tb[x] = (((lo_b[lane] >> z) & 1) + 2 * ((hi_b[lane] >> z) & 1)) as u8;
                    }
                    let rc = y == 0 && ((RC[r] >> z) & 1) == 1;
                    let chi_row = KeccakfChiTableSM::calculate_table_row(&ta, &tb, rc);
                    chi_hist[chi_row as usize] += 1;

                    // The committed accumulator holds the packed lookup INPUT
                    // (base 28), NOT the compact table-row index (base 16)
                    chi_accs[z] = KeccakfChiTableSM::calculate_table_input(&ta, &tb, rc);
                }

                // On narrow layouts the packed χ-inputs of χ-row group y are
                // committed at its anchor row, the group-row holding lane 5y.
                // NOTE: chi_acc only exists for lanes_per_row < 25; comment out
                //       when instantiating the wide layout.
                trace[group + (5 * y) / LANES_PER_ROW].set_all_chi_acc(&chi_accs);
            }

            // Advance both instances one round
            state_a = chi_iota(&lo_a, r);
            state_b = chi_iota(&lo_b, r);
        }

        // Boundary output groups: plain bits of the final states
        Self::set_state_group(trace, GROUP_OUT_A, &state_a, &[0u64; LANES]);
        Self::set_state_group(trace, GROUP_OUT_B, &state_b, &[0u64; LANES]);
    }

    /// Writes a sliced state a + SLOT·b into the ROWS_PER_STATE rows of a group:
    /// group-row y holds plane y, with lane x at columns [64x, 64x+64).
    #[inline(always)]
    fn set_state_group<R: KeccakfTraceRowOps<F>>(
        trace: &mut [R],
        first_row: usize,
        a: &LaneState,
        b: &LaneState,
    ) {
        for k in 0..ROWS_PER_STATE {
            let mut cells = [0u8; BITS_PER_ROW];
            for lane in 0..LANES_PER_ROW {
                let (la, lb) = (a[k * LANES_PER_ROW + lane], b[k * LANES_PER_ROW + lane]);
                for z in 0..LANE_BITS {
                    cells[lane * LANE_BITS + z] =
                        (((la >> z) & 1) + SLOT as u64 * ((lb >> z) & 1)) as u8;
                }
            }
            trace[first_row + k].set_all_state(&cells);
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
    pub fn compute_witness<R: KeccakfTraceRowOps<F>, const NUM_ROWS: usize, const AIR_ID: usize>(
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
mod lane_native_tests {
    use super::*;
    use zisk_precomp_helpers::{keccak_f, keccakf_bit_pos, keccakf_state_from_linear};

    fn sample(seed: u64) -> LaneState {
        let mut s = [0u64; LANES];
        let mut x = seed | 1;
        for lane in s.iter_mut() {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *lane = x;
        }
        s
    }

    fn to_linear(state: &[[[u8; 64]; 5]; 5]) -> LaneState {
        let mut out = [0u64; LANES];
        for x in 0..5 {
            for y in 0..5 {
                for z in 0..64 {
                    out[x + 5 * y] |= (state[x][y][z] as u64) << z;
                }
            }
        }
        out
    }

    /// The lane-native round chain must reproduce the reference permutation.
    #[test]
    fn round_chain_matches_reference() {
        for seed in 1..20u64 {
            let start = sample(seed);
            let mut reference = keccakf_state_from_linear(&start);
            keccak_f(&mut reference);

            let mut state = start;
            for r in 0..ROUNDS {
                let cols = ThetaColumns::from_state(&state);
                let (lo, _) = theta_rho_pi(&state, &cols.parities);
                state = chi_iota(&lo, r);
            }
            assert_eq!(state, to_linear(&reference), "seed {seed}");
        }
    }

    /// The two θ bit-planes must reproduce the old per-position formula
    /// `state + parity(x-1) + parity(x+1, z-1)` at the ρπ source.
    #[test]
    fn theta_planes_match_positionwise() {
        for seed in 1..10u64 {
            let state = sample(seed);
            let bits = keccakf_state_from_linear(&state);
            let cols = ThetaColumns::from_state(&state);
            let (lo, hi) = theta_rho_pi(&state, &cols.parities);
            let parity = |x: usize, z: usize| ((cols.parities[x] >> z) & 1) as u8;

            for y in 0..5 {
                for x in 0..5 {
                    for z in 0..64 {
                        let (sx, sy) = ((x + 3 * y) % 5, x);
                        let sz = (z + 64 - RHO_OFFSETS[sx][sy]) % 64;
                        let expected = bits[sx][sy][sz]
                            + parity((sx + 4) % 5, sz)
                            + parity((sx + 1) % 5, (sz + 63) % 64);
                        let lane = x + 5 * y;
                        let got = (((lo[lane] >> z) & 1) + 2 * ((hi[lane] >> z) & 1)) as u8;
                        assert_eq!(got, expected, "seed {seed} ({x},{y},{z})");
                    }
                }
            }
        }
    }

    /// Column sums must match the naive five-bit count, and parity its low bit.
    #[test]
    fn column_sums_match_naive() {
        for seed in 1..10u64 {
            let state = sample(seed);
            let cols = ThetaColumns::from_state(&state);
            for x in 0..5 {
                for z in 0..64 {
                    let expected: u8 =
                        (0..5).map(|y| ((state[x + 5 * y] >> z) & 1) as u8).sum();
                    assert_eq!(cols.sum_at(x, z), expected, "({x},{z})");
                    assert_eq!(((cols.parities[x] >> z) & 1) as u8, expected % 2);
                }
            }
        }
    }

    /// The row-major cell layout must match `keccakf_bit_pos` on the old path.
    #[test]
    fn state_group_layout_matches_bit_pos() {
        let (a, b) = (sample(7), sample(11));
        let mut expected = [0u8; WIDTH];
        for x in 0..5 {
            for y in 0..5 {
                for z in 0..64 {
                    let lane = x + 5 * y;
                    expected[keccakf_bit_pos(x, y, z)] = (((a[lane] >> z) & 1)
                        + SLOT as u64 * ((b[lane] >> z) & 1))
                        as u8;
                }
            }
        }
        for k in 0..ROWS_PER_STATE {
            let mut cells = [0u8; BITS_PER_ROW];
            for lane in 0..LANES_PER_ROW {
                let (la, lb) = (a[k * LANES_PER_ROW + lane], b[k * LANES_PER_ROW + lane]);
                for z in 0..LANE_BITS {
                    cells[lane * LANE_BITS + z] =
                        (((la >> z) & 1) + SLOT as u64 * ((lb >> z) & 1)) as u8;
                }
            }
            assert_eq!(cells[..], expected[k * BITS_PER_ROW..(k + 1) * BITS_PER_ROW]);
        }
    }
}
