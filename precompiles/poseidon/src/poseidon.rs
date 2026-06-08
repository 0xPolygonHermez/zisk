use core::panic;
use std::sync::Arc;

use fields::{
    add, matmul_external, pow7, pow7add, prodadd, Poseidon1Constants, Poseidon1_16,
    Poseidon2Constants, Poseidon2_16, PrimeField64,
};
use rayon::prelude::*;

use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult, SetupCtx};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use zisk_common::{OperationPoseidonData, OP};
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{PoseidonTrace, PoseidonTraceRow, PoseidonTraceRowOps};

/// Per-operation input record assembled from the bus payload.
///
/// `is_poseidon1` records which hash family this operation belongs to (the two
/// families share a single bus-data payload shape and op-type, and are
/// distinguished here by the `OP` field). The trace fill is currently identical
/// for both families (the untouched Poseidon2 AIR layout); the flag is the hook
/// for the family-distinguishing constraints to be added later.
#[derive(Debug)]
pub struct PoseidonInput {
    pub step_main: u64,
    pub addr_main: u32,
    pub state: [u64; 16],
    pub is_poseidon1: bool,
}

impl PoseidonInput {
    pub fn from(values: &OperationPoseidonData<u64>) -> Self {
        Self {
            step_main: values[4],
            addr_main: values[3] as u32,
            state: values[5..21].try_into().unwrap(),
            is_poseidon1: values[OP] == ZiskOp::Poseidon1 as u64,
        }
    }
}

/// The `PoseidonSM` struct encapsulates the logic of the Poseidon State Machine,
/// serving both the Poseidon1 and Poseidon2 hash families.
pub struct PoseidonSM<F: PrimeField64> {
    /// Number of available poseidon permutations in the trace.
    pub num_available_poseidons: usize,
    _phantom: std::marker::PhantomData<F>,
}

pub const CLOCKS: usize = 14;

impl<F: PrimeField64> PoseidonSM<F> {
    /// Creates a new Poseidon State Machine instance.
    ///
    /// The `_std` parameter is unused (Poseidon has no Std interaction at
    /// construction); it exists to keep the constructor signature uniform
    /// with the other uniform precompiles, so the `zisk_precompile!` macro
    /// can call `PoseidonSM::new(std)` like the others.
    pub fn new(_std: Arc<Std<F>>) -> Arc<Self> {
        // Compute some useful values
        let num_available_poseidons = PoseidonTrace::<PoseidonTraceRow<F>>::NUM_ROWS / CLOCKS - 1;

        Arc::new(Self { num_available_poseidons, _phantom: std::marker::PhantomData })
    }

    /// Processes a slice of operation data, updating the trace and multiplicities.
    ///
    /// # Arguments
    /// * `trace` - A mutable reference to the Poseidon2 trace.
    /// * `num_circuits` - The number of circuits to process.
    /// * `input` - The operation data to process.
    /// * `multiplicity` - A mutable slice to update with multiplicities for the operation.
    #[inline(always)]
    pub fn process_input<R: PoseidonTraceRowOps<F>>(
        &self,
        trace: &mut [R],
        input: &PoseidonInput,
        is_active: bool,
    ) {
        // Fill the per-clock round states for the selected hash family. Both
        // families share the 14-clock row layout (see poseidon.pil); only the
        // round computation differs.
        let sel_poseidon1 = input.is_poseidon1;
        let round_states = if sel_poseidon1 {
            Self::compute_round_states_poseidon1(&input.state)
        } else {
            Self::compute_round_states_poseidon2(&input.state)
        };

        for r in 0..CLOCKS {
            let mut chunks = [[0u32; 2]; 16];
            for i in 0..16 {
                let state = round_states[r][i];
                chunks[i][0] = state as u32;
                chunks[i][1] = (state >> 32) as u32;
            }
            trace[r].set_all_chunks(&chunks);
            trace[r].set_sel_poseidon1(sel_poseidon1);
        }

        if !is_active {
            return;
        }

        // Fill step and addr
        trace[0].set_step_addr(input.step_main);
        trace[1].set_step_addr(input.addr_main as u64);

        // Fill in_use
        for item in trace.iter_mut().take(CLOCKS) {
            item.set_in_use(true);
        }
    }

    /// Computes the 14 per-clock round states for the Poseidon2 permutation,
    /// laid out to match the Poseidon2 constraints in poseidon.pil.
    #[inline(always)]
    fn compute_round_states_poseidon2(input_state: &[u64; 16]) -> [[u64; 16]; CLOCKS] {
        let mut round_states = [[0u64; 16]; CLOCKS];
        round_states[0] = *input_state;

        let mut state = input_state.map(|x| F::from_u64(x));
        matmul_external::<F, 16>(&mut state);
        round_states[1] = state.map(|x| x.as_canonical_u64());

        for r in 0..Poseidon2_16::HALF_ROUNDS {
            let mut c_slice = [F::ZERO; 16];
            for (i, c) in c_slice.iter_mut().enumerate() {
                *c = F::from_u64(Poseidon2_16::RC[r * 16 + i]);
            }
            pow7add::<F, 16>(&mut state, &c_slice);
            matmul_external::<F, 16>(&mut state);
            round_states[2 + r] = state.map(|x| x.as_canonical_u64());
        }

        let mut row = 6;
        let mut index = 0;
        for r in 0..Poseidon2_16::N_PARTIAL_ROUNDS {
            round_states[row][index] = state[0].as_canonical_u64();
            index += 1;

            state[0] += F::from_u64(Poseidon2_16::RC[Poseidon2_16::HALF_ROUNDS * 16 + r]);
            state[0] = pow7(state[0]);
            let sum = add::<F, 16>(&state);
            prodadd::<F, 16>(&mut state, Poseidon2_16::DIAG, sum);
            if r == 10 {
                round_states[7] = state.map(|x| x.as_canonical_u64());
                row = 8;
                index = 0;
            }
        }

        round_states[9] = state.map(|x| x.as_canonical_u64());

        for r in 0..Poseidon2_16::HALF_ROUNDS {
            let mut c_slice = [F::ZERO; 16];
            for (i, c) in c_slice.iter_mut().enumerate() {
                *c = F::from_u64(
                    Poseidon2_16::RC[Poseidon2_16::HALF_ROUNDS * 16
                        + Poseidon2_16::N_PARTIAL_ROUNDS
                        + r * 16
                        + i],
                );
            }
            pow7add::<F, 16>(&mut state, &c_slice);
            matmul_external::<F, 16>(&mut state);
            round_states[10 + r] = state.map(|x| x.as_canonical_u64());
        }

        round_states
    }

    /// Computes the 14 per-clock round states for the Poseidon1 (Hades)
    /// permutation, laid out to match the Poseidon1 constraints in poseidon.pil:
    ///
    /// ```text
    ///   row 0 : input
    ///   row 1 : input + C[0..n]                         (initial ARC)
    ///   row 2..4 : M·(pow7(prev) + C-slice)             (3 full rounds)
    ///   row 5 : P·(pow7(row4) + C[HALF*n..])            (transition)
    ///   row 6 : anchors s[0] of partial rounds 0..10
    ///   row 7 : state after partial round 10
    ///   row 8 : anchors s[0] of partial rounds 11..21
    ///   row 9 : state after partial round 21
    ///   row 10..12 : M·(pow7(prev) + C[post..])         (3 full rounds)
    ///   row 13 : M·(pow7(row12))                        (final, no ARC)
    /// ```
    #[inline(always)]
    fn compute_round_states_poseidon1(input_state: &[u64; 16]) -> [[u64; 16]; CLOCKS] {
        const W: usize = 16;
        const HALF: usize = 4; // Poseidon1_16::HALF_FULL_ROUNDS
        const NP: usize = 22; // Poseidon1_16::N_PARTIAL_ROUNDS
        let post: usize = (HALF + 1) * W + NP;
        let partial_c: usize = (HALF + 1) * W;
        let s_stride: usize = 2 * W - 1;

        // state[i] = sum_j old[j] * MAT[j*W + i]
        let matmul = |mat: &[u64], st: &mut [F; W]| {
            let old = *st;
            for i in 0..W {
                let mut sum = old[0] * F::from_u64(mat[i]);
                for j in 1..W {
                    sum += old[j] * F::from_u64(mat[j * W + i]);
                }
                st[i] = sum;
            }
        };

        let mut round_states = [[0u64; W]; CLOCKS];
        round_states[0] = *input_state;

        let mut state = input_state.map(F::from_u64);

        // row 1: initial ARC.
        for (i, s) in state.iter_mut().enumerate() {
            *s += F::from_u64(Poseidon1_16::C[i]);
        }
        round_states[1] = state.map(|x| x.as_canonical_u64());

        // rows 2..4: 3 full rounds with M, ARC after the S-box.
        for r in 0..(HALF - 1) {
            for (i, s) in state.iter_mut().enumerate() {
                *s = pow7(*s) + F::from_u64(Poseidon1_16::C[(r + 1) * W + i]);
            }
            matmul(Poseidon1_16::M, &mut state);
            round_states[2 + r] = state.map(|x| x.as_canonical_u64());
        }

        // row 5: transition full round with P.
        for (i, s) in state.iter_mut().enumerate() {
            *s = pow7(*s) + F::from_u64(Poseidon1_16::C[HALF * W + i]);
        }
        matmul(Poseidon1_16::P, &mut state);
        round_states[5] = state.map(|x| x.as_canonical_u64());

        // rows 6/8: 22 partial rounds, anchors packed 11 + 11.
        let mut row = 6;
        let mut index = 0;
        for r in 0..NP {
            // anchor s[0] (pre-S-box) of this round
            round_states[row][index] = state[0].as_canonical_u64();
            index += 1;

            let a = pow7(state[0]) + F::from_u64(Poseidon1_16::C[partial_c + r]);
            let sb = s_stride * r;

            // s0_new = a*S[sb] + sum_{j>=1} state[j]*S[sb+j]
            let mut s0_new = a * F::from_u64(Poseidon1_16::S[sb]);
            for (j, s) in state.iter().enumerate().skip(1) {
                s0_new += *s * F::from_u64(Poseidon1_16::S[sb + j]);
            }
            // state[t] += a*S[sb + (W-1) + t] for t = 1..W
            for (t, s) in state.iter_mut().enumerate().skip(1) {
                *s += a * F::from_u64(Poseidon1_16::S[sb + (W - 1) + t]);
            }
            state[0] = s0_new;

            if r == 10 {
                round_states[7] = state.map(|x| x.as_canonical_u64());
                row = 8;
                index = 0;
            }
        }
        round_states[9] = state.map(|x| x.as_canonical_u64());

        // rows 10..12: 3 full rounds with M; row 13: final round with M (no ARC).
        for r in 0..(HALF - 1) {
            for (i, s) in state.iter_mut().enumerate() {
                *s = pow7(*s) + F::from_u64(Poseidon1_16::C[post + r * W + i]);
            }
            matmul(Poseidon1_16::M, &mut state);
            round_states[10 + r] = state.map(|x| x.as_canonical_u64());
        }
        for s in state.iter_mut() {
            *s = pow7(*s);
        }
        matmul(Poseidon1_16::M, &mut state);
        round_states[13] = state.map(|x| x.as_canonical_u64());

        round_states
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `sctx` - The setup context containing the setup data.
    /// * `inputs` - A slice of operations to process.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed witness data.
    pub fn compute_witness<R: PoseidonTraceRowOps<F>>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<PoseidonInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut poseidon2_trace = PoseidonTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let num_rows = poseidon2_trace.num_rows();
        let num_available_poseidons = self.num_available_poseidons;

        // Check that we can fit all the poseidons in the trace
        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        let num_rows_needed = if num_inputs < num_available_poseidons {
            num_inputs * CLOCKS
        } else if num_inputs == num_available_poseidons {
            num_rows
        } else {
            panic!(
                "Exceeded available Poseidon inputs: requested {}, but only {} are available.",
                num_inputs, self.num_available_poseidons
            );
        };

        tracing::debug!(
            "··· Creating Poseidon2 instance [{}{{}} / {} rows filled {:.2}%]",
            num_rows_needed,
            num_rows,
            (num_rows_needed as f64 / num_rows as f64 * 100.0) as usize
        );

        timer_start_trace!(POSEIDON2_TRACE);
        let mut trace_rows = poseidon2_trace.buffer.as_mut_slice();
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
        par_traces.into_par_iter().enumerate().for_each(|(index, trace)| {
            let input_index = inputs_indexes[index];
            let input = &inputs[input_index.0][input_index.1];
            self.process_input::<R>(trace, input, true);
        });

        timer_stop_and_log_trace!(POSEIDON2_TRACE);

        timer_start_trace!(POSEIDON2_PADDING);

        // 3] Fill the padding rows with Poseidon2(0)
        let padding_rows_start = num_rows_needed;
        let padding_rows_end: usize =
            padding_rows_start + ((num_available_poseidons - num_inputs) * CLOCKS);

        // Split the padding trace into padding chunks
        let padding_trace = &mut poseidon2_trace.buffer[padding_rows_start..padding_rows_end];
        let mut padding_chunks: Vec<_> = padding_trace.chunks_mut(CLOCKS).collect();

        // Process padding in parallel
        if let Some((first, rest)) = padding_chunks.split_first_mut() {
            self.process_input::<R>(
                first,
                &PoseidonInput { state: [0; 16], step_main: 0, addr_main: 0, is_poseidon1: false },
                false,
            );

            rest.par_iter_mut().for_each(|chunk| {
                chunk.copy_from_slice(first);
            });
        }

        // 4] The non-usable rows should be zeroes, which are already set at initialization

        timer_stop_and_log_trace!(POSEIDON2_PADDING);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut poseidon2_trace)))
    }
}
