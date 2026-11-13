//! Witness of the `KeccakfProxy` air: one row per Keccak-f invocation.
//!
//! `Keccakf` proves every *unique* permutation of the execution once; this air proves the
//! invocations, i.e. the memory accesses and the operation the main processor sees. A row is an
//! *original* (the first invocation of its permutation) or a *replica* (a later invocation with
//! the same input state). An original takes its permutation from `Keccakf` and offers it to its
//! replicas, so it carries how many replicas there are: that count is only known once the whole
//! execution has been seen, and it is handed here as a table indexed by unique permutation.

use std::marker::PhantomData;
use std::sync::Arc;

use proofman_common::{AirInstance, FromTrace, GenericTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use tiny_keccak::keccakf;
use zisk_core::{addr_hi, keccakf_dedup_index, keccakf_dedup_is_replica};
use zisk_pil::{KeccakfProxyTraceRowOps, ZISK_AIRGROUP_ID};

use crate::{keccakf_constants::LANES, KeccakfInput};

/// 32-bit limbs of a state: two per lane, low limb first.
const LIMBS_PER_STATE: usize = 2 * LANES;

/// The `KeccakfProxySM` struct encapsulates the logic of the KeccakfProxy State Machine.
pub struct KeccakfProxySM<F: PrimeField64> {
    _phantom: PhantomData<F>,
}

#[inline(always)]
fn state_limbs(state: &[u64; LANES]) -> [u32; LIMBS_PER_STATE] {
    let mut limbs = [0u32; LIMBS_PER_STATE];
    for (lane, value) in state.iter().enumerate() {
        limbs[2 * lane] = *value as u32;
        limbs[2 * lane + 1] = (*value >> 32) as u32;
    }
    limbs
}

impl<F: PrimeField64> KeccakfProxySM<F> {
    /// Creates a new KeccakfProxy State Machine instance.
    pub fn new() -> Arc<Self> {
        Arc::new(Self { _phantom: PhantomData })
    }

    /// Fills the row of one invocation.
    ///
    /// `replicas` is the number of replicas of the permutation, read only when the invocation
    /// is the original.
    #[inline(always)]
    pub fn process_input<R: KeccakfProxyTraceRowOps<F>>(
        row: &mut R,
        input: &KeccakfInput,
        replicas: u32,
    ) {
        let is_replica = keccakf_dedup_is_replica(input.dedup_word);
        let mut state_out = input.state;
        keccakf(&mut state_out);

        row.set_in_use(true);
        row.set_is_replica(is_replica);
        row.set_replicas(if is_replica { 0 } else { replicas });
        row.set_step(input.step_main);
        row.set_addr(input.addr_main);
        row.set_addr_hi(addr_hi(input.addr_main) == 1);
        row.set_all_state_in(&state_limbs(&input.state));
        row.set_all_state_out(&state_limbs(&state_out));
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    ///
    /// # Arguments
    /// * `inputs` - The invocations to process, originals and replicas.
    /// * `replicas` - Number of replicas of every unique permutation of the execution, indexed
    ///   by the index of the unique permutation (see `zisk_core::keccakf_dedup`).
    pub fn compute_witness<
        R: KeccakfProxyTraceRowOps<F> + Send,
        const NUM_ROWS: usize,
        const AIR_ID: usize,
    >(
        &self,
        inputs: &[Vec<KeccakfInput>],
        replicas: &[u32],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = GenericTrace::<R, NUM_ROWS, ZISK_AIRGROUP_ID, AIR_ID>::new_from_vec_zeroes(
            trace_buffer,
        )?;
        let num_rows = trace.num_rows();

        let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
        if num_inputs > num_rows {
            panic!(
                "Exceeded available KeccakfProxy inputs: requested {num_inputs}, but only {num_rows} are available."
            );
        }

        tracing::debug!(
            "··· Creating KeccakfProxy instance [{} / {} rows filled {:.2}%] [{} replicas]",
            num_inputs,
            num_rows,
            num_inputs as f64 / num_rows as f64 * 100.0,
            inputs.iter().flatten().filter(|i| keccakf_dedup_is_replica(i.dedup_word)).count(),
        );

        let flat_inputs: Vec<&KeccakfInput> = inputs.iter().flatten().collect();
        trace.buffer[..num_inputs].par_iter_mut().zip(flat_inputs.par_iter()).for_each(
            |(row, input)| {
                // A permutation without a count has no replicas: the table only grows up to the
                // last permutation that has one
                let index = keccakf_dedup_index(input.dedup_word) as usize;
                let replicas = replicas.get(index).copied().unwrap_or(0);
                Self::process_input(row, input, replicas);
            },
        );

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_core::KECCAKF_DEDUP_REPLICA_FLAG;
    use zisk_pil::KeccakfProxyTraceRow;

    #[test]
    fn rows_carry_the_permutation_and_only_originals_the_replica_count() {
        let state: [u64; LANES] =
            core::array::from_fn(|i| 0x0123_4567_89ab_cdef ^ (i as u64) << 40);
        let mut output = state;
        keccakf(&mut output);

        let original =
            KeccakfInput { step_main: 77, addr_main: 0x1_2345_6780, state, dedup_word: 3 };
        let replica = KeccakfInput {
            step_main: 99,
            addr_main: 0xc000_1000,
            state,
            dedup_word: 3 | KECCAKF_DEDUP_REPLICA_FLAG,
        };

        let mut row = KeccakfProxyTraceRow::<Goldilocks>::default();
        KeccakfProxySM::<Goldilocks>::process_input(&mut row, &original, 5);
        assert!(row.get_in_use() && !row.get_is_replica() && row.get_addr_hi());
        assert_eq!((row.get_replicas(), row.get_step(), row.get_addr()), (5, 77, 0x1_2345_6780));
        assert_eq!(row.get_all_state_in(), state_limbs(&state));
        assert_eq!(row.get_all_state_out(), state_limbs(&output));

        let mut row = KeccakfProxyTraceRow::<Goldilocks>::default();
        KeccakfProxySM::<Goldilocks>::process_input(&mut row, &replica, 5);
        assert!(row.get_in_use() && row.get_is_replica() && !row.get_addr_hi());
        assert_eq!((row.get_replicas(), row.get_step(), row.get_addr()), (0, 99, 0xc000_1000));
        assert_eq!(row.get_all_state_out(), state_limbs(&output));
    }
}
