use std::sync::Arc;

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;

use proofman_common::{AirInstance, FromTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

use zisk_common::OperationKeccakData;
use zisk_pil::{KeccakfTrace, KeccakfTraceRowOps};
use zisk_precomp_helpers::{
    keccak_f_round, keccakf_bit_pos, keccakf_state_from_linear, KeccakState,
};

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
struct ThetaColumns {
    sums: [[u8; 64]; 5],
    parities: [[u8; 64]; 5],
}

impl ThetaColumns {
    fn from_state(state: &KeccakState) -> Self {
        let mut sums = [[0u8; 64]; 5];
        let mut parities = [[0u8; 64]; 5];
        for x in 0..5 {
            for z in 0..64 {
                let sum = state[x][0][z]
                    + state[x][1][z]
                    + state[x][2][z]
                    + state[x][3][z]
                    + state[x][4][z];
                sums[x][z] = sum;
                parities[x][z] = sum % 2;
            }
        }
        Self { sums, parities }
    }

    /// θ-output at the ρπ-source of χ-position (x, y, z): a clean state bit plus
    /// two clean parities, value in [0,3]. Mirrors b = π(ρ(θ(state))).
    #[inline(always)]
    fn theta_out_at_source(&self, state: &KeccakState, x: usize, y: usize, z: usize) -> u8 {
        let sx = (x + 3 * y) % 5;
        let sy = x;
        let sz = (z + 64 - RHO_OFFSETS[sx][sy]) % 64;
        state[sx][sy][sz]
            + self.parities[(sx + 4) % 5][sz]
            + self.parities[(sx + 1) % 5][(sz + 63) % 64]
    }
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

        // Convert input states to 5x5x64 representation
        let zero_state = [0u64; 25];
        let mut state_a = keccakf_state_from_linear(&input_a.state);
        let mut state_b = keccakf_state_from_linear(input_b.map_or(&zero_state, |b| &b.state));

        // Boundary input groups: plain bits
        Self::set_bits_group(trace, GROUP_IN_A, &state_a);
        Self::set_bits_group(trace, GROUP_IN_B, &state_b);

        // Round groups
        let mut cells = [0u8; WIDTH];
        let mut ta = [0u8; 5];
        let mut tb = [0u8; 5];
        // let mut chi_accs = [0u32; LANE_BITS];
        for r in 0..=ROUNDS {
            // Sliced state-group of round r
            let group = GROUP_ROUND_0 + r * ROWS_PER_STATE;
            for x in 0..5 {
                for y in 0..5 {
                    for z in 0..64 {
                        cells[keccakf_bit_pos(x, y, z)] =
                            state_a[x][y][z] + SLOT * state_b[x][y][z];
                    }
                }
            }
            Self::set_group(trace, group, &cells);

            if r == ROUNDS {
                break;
            }

            // θ columns of both instances
            let cols_a = ThetaColumns::from_state(&state_a);
            let cols_b = ThetaColumns::from_state(&state_b);

            // Committed sliced parities: position p = x·64+z lives at group-row
            // p / C_PER_ROW, column p % C_PER_ROW
            for row in 0..ROWS_PER_STATE {
                let mut c_cells = [0u8; C_PER_ROW];
                for (j, c_cell) in c_cells.iter_mut().enumerate() {
                    let pos = row * C_PER_ROW + j;
                    if pos < 320 {
                        let (x, z) = (pos / 64, pos % 64);
                        *c_cell = cols_a.parities[x][z] + SLOT * cols_b.parities[x][z];
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
                        ta[x] = cols_a.theta_out_at_source(&state_a, x, y, z);
                        tb[x] = cols_b.theta_out_at_source(&state_b, x, y, z);
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
            keccak_f_round(&mut state_a, r);
            keccak_f_round(&mut state_b, r);
            Self::reduce_mod2(&mut state_a);
            Self::reduce_mod2(&mut state_b);
        }

        // Boundary output groups: plain bits of the final states
        Self::set_bits_group(trace, GROUP_OUT_A, &state_a);
        Self::set_bits_group(trace, GROUP_OUT_B, &state_b);
    }

    /// Writes 1600 lane-major cells into the ROWS_PER_STATE rows of a state-group:
    /// group-row k holds the lanes [k·LANES_PER_ROW, (k+1)·LANES_PER_ROW).
    #[inline(always)]
    fn set_group<R: KeccakfTraceRowOps<F>>(trace: &mut [R], first_row: usize, cells: &[u8; WIDTH]) {
        for k in 0..ROWS_PER_STATE {
            let row_cells: &[u8; BITS_PER_ROW] =
                cells[k * BITS_PER_ROW..(k + 1) * BITS_PER_ROW].try_into().unwrap();
            trace[first_row + k].set_all_state(row_cells);
        }
    }

    /// Writes a clean (bit-valued) state into one boundary group.
    #[inline(always)]
    fn set_bits_group<R: KeccakfTraceRowOps<F>>(
        trace: &mut [R],
        first_row: usize,
        state: &KeccakState,
    ) {
        let mut cells = [0u8; WIDTH];
        for x in 0..5 {
            for y in 0..5 {
                for z in 0..64 {
                    cells[keccakf_bit_pos(x, y, z)] = state[x][y][z];
                }
            }
        }
        Self::set_group(trace, first_row, &cells);
    }

    #[inline(always)]
    fn reduce_mod2(state: &mut KeccakState) {
        state.iter_mut().flatten().flatten().for_each(|bit| *bit %= 2);
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness<R: KeccakfTraceRowOps<F>>(
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
