//! Trace generation: one 128-row block per call, filled from the round schedule.
//!
//! Inactive blocks run the permutation on a zero state with `in_use = 0`, so every
//! constraint holds and no bus activity is emitted. The U16 range-check multiplicities
//! count all blocks because the lookups are unconditional in the AIR.

use std::sync::Arc;

use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanError, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use zisk_common::OperationKoalaPoseidon2Data;
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
use zisk_koala_poseidon2_foundation::{rounds::Rounds, validate_state, MODULUS};
use zisk_pil::{KoalaPoseidon2Trace, KoalaPoseidon2TraceRowOps};

pub use zisk_koala_poseidon2_foundation::rounds::CLOCKS;

/// One call as taken from the operation bus: step, operand address and the eight input words.
#[derive(Debug)]
pub struct KoalaPoseidon2Input {
    pub(crate) step: u64,
    pub(crate) address: u32,
    pub(crate) state: [u64; 8],
}

impl KoalaPoseidon2Input {
    /// Decodes and validates a bus payload; malformed payloads panic before any trace is written.
    pub fn from(values: &OperationKoalaPoseidon2Data<u64>) -> Self {
        assert_eq!(values[0], ZiskOp::KoalaPoseidon2 as u64);
        assert_eq!(values[1], ZiskOperationType::KoalaPoseidon2 as u64);
        assert_eq!(values[2], 0, "KoalaPoseidon2 has no a operand");
        let address = u32::try_from(values[3]).expect("KoalaPoseidon2 address exceeds 32 bits");
        checked_address(address);
        assert!(values[4] < (1 << 40), "KoalaPoseidon2 step exceeds 40 bits");
        validate_state(&std::array::from_fn(|i| (values[5 + i / 2] >> (32 * (i % 2))) as u32))
            .expect("noncanonical KoalaPoseidon2 payload");
        Self { step: values[4], address, state: values[5..].try_into().unwrap() }
    }
}

pub(crate) fn checked_address(address: u32) {
    assert_eq!(address % 8, 0, "KoalaPoseidon2 address must be 8-byte aligned");
    address.checked_add(63).expect("KoalaPoseidon2 memory span exceeds 32 bits");
}

struct RoundWitness {
    rounds: Rounds,
}

impl RoundWitness {
    fn pinned() -> Self {
        Self { rounds: Rounds::pinned() }
    }

    /// Fills one block and accumulates its limb range-check counts.
    fn fill_block<F: PrimeField64, R: KoalaPoseidon2TraceRowOps<F>>(
        &self,
        rows: &mut [R],
        input: &KoalaPoseidon2Input,
        active: bool,
        range_checks: &mut [u32],
    ) -> ProofmanResult<()> {
        if rows.len() != CLOCKS || range_checks.len() != 1 << 16 {
            return Err(ProofmanError::InvalidParameters(
                "invalid KoalaPoseidon2 block dimensions".into(),
            ));
        }
        checked_address(input.address);
        let state = std::array::from_fn(|i| (input.state[i / 2] >> (32 * (i % 2))) as u32);
        self.rounds
            .execute(state, |clock, source| {
                let row = &mut rows[clock];
                let limbs = source.state.map(split);
                let gap = source.state.map(|v| split(MODULUS - 1 - v));
                let quotient = source.quotient.map(split);
                row.set_all_limbs(&limbs);
                row.set_all_gap(&gap);
                row.set_all_quotient(&quotient);
                row.set_all_base(&source.base);
                row.set_in_use(active);
                row.set_addr(input.address);
                row.set_main_step(input.step);
                for family in [limbs, gap, quotient] {
                    for value in family.into_iter().flatten() {
                        range_checks[usize::from(value)] += 1;
                    }
                }
            })
            .map_err(ProofmanError::InvalidParameters)?;
        Ok(())
    }

    /// Fills active blocks in parallel, pads with inactive blocks, and returns the U16 multiplicities.
    fn fill_rows<F: PrimeField64, R: KoalaPoseidon2TraceRowOps<F>>(
        &self,
        rows: &mut [R],
        inputs: &[Vec<KoalaPoseidon2Input>],
    ) -> ProofmanResult<Vec<u32>> {
        let inputs: Vec<_> = inputs.iter().flatten().collect();
        let capacity = rows.len() / CLOCKS;
        if inputs.len() > capacity
            || rows.len() % CLOCKS != 0
            || rows.len() > KoalaPoseidon2Trace::<()>::NUM_ROWS
        {
            return Err(ProofmanError::InvalidParameters(
                "KoalaPoseidon2 capacity exceeded".into(),
            ));
        }
        let padding = KoalaPoseidon2Input { step: 0, address: 0, state: [0; 8] };
        let mut padding_rows = vec![R::default(); CLOCKS];
        let mut padding_counts = vec![0_u32; 1 << 16];
        self.fill_block::<F, R>(&mut padding_rows, &padding, false, &mut padding_counts)?;
        let (active_rows, inactive_rows) = rows.split_at_mut(inputs.len() * CLOCKS);
        inactive_rows.par_chunks_mut(CLOCKS).for_each(|block| block.copy_from_slice(&padding_rows));
        let mut counts = active_rows
            .par_chunks_mut(CLOCKS)
            .zip(inputs.par_iter())
            .try_fold(
                || vec![0_u32; 1 << 16],
                |mut counts, (block, input)| {
                    self.fill_block::<F, R>(block, input, true, &mut counts)?;
                    Ok::<_, ProofmanError>(counts)
                },
            )
            .try_reduce(
                || vec![0_u32; 1 << 16],
                |mut total, counts| {
                    for (a, b) in total.iter_mut().zip(counts) {
                        *a += b;
                    }
                    Ok(total)
                },
            )?;
        let padding_blocks = (capacity - inputs.len()) as u32;
        for (total, padding) in counts.iter_mut().zip(padding_counts) {
            *total += padding_blocks * padding;
        }
        Ok(counts)
    }
}

fn split(value: u32) -> [u16; 2] {
    [value as u16, (value >> 16) as u16]
}

pub struct KoalaPoseidon2SM<F: PrimeField64> {
    witness: RoundWitness,
    std: Arc<Std<F>>,
    range_id: usize,
}

impl<F: PrimeField64> KoalaPoseidon2SM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let range_id =
            std.get_range_id(0, (1 << 16) - 1, None).expect("KoalaPoseidon2 U16 range missing");
        Arc::new(Self { witness: RoundWitness::pinned(), std, range_id })
    }

    pub fn compute_witness<R: KoalaPoseidon2TraceRowOps<F>>(
        &self,
        _sctx: &SetupCtx<F>,
        inputs: &[Vec<KoalaPoseidon2Input>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = KoalaPoseidon2Trace::<R>::new_from_vec_zeroes(trace_buffer)?;
        let counts = self.witness.fill_rows::<F, R>(&mut trace.buffer, inputs)?;
        self.std.range_check_ranged(self.range_id, None, &counts);
        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_definitions::koala_poseidon2::permute_packed;
    use zisk_pil::{KoalaPoseidon2TraceRow, KoalaPoseidon2TraceRowPacked};

    fn input(index: usize) -> KoalaPoseidon2Input {
        let canonical: [u32; 16] = std::array::from_fn(|lane| {
            if (lane + index) % 2 == 0 {
                MODULUS - 1
            } else {
                (lane + index) as u32
            }
        });
        KoalaPoseidon2Input {
            step: (1 << 39) + index as u64,
            address: 0xa000_1000 + 64 * index as u32,
            state: std::array::from_fn(|i| {
                u64::from(canonical[2 * i]) | (u64::from(canonical[2 * i + 1]) << 32)
            }),
        }
    }

    fn check_rows<R: KoalaPoseidon2TraceRowOps<Goldilocks>>(
        count: usize,
        blocks: usize,
    ) -> Vec<u32> {
        let witness = RoundWitness::pinned();
        let mut rows = vec![R::default(); blocks * CLOCKS];
        let inputs = vec![(0..count).map(input).collect::<Vec<_>>()];
        let counts = witness.fill_rows::<Goldilocks, R>(&mut rows, &inputs).unwrap();
        let mut observed = vec![0_u32; 1 << 16];
        for block in 0..blocks {
            let active = inputs[0].get(block);
            let mut expected = active.map_or([0; 8], |v| v.state);
            let state = std::array::from_fn(|i| (expected[i / 2] >> (32 * (i % 2))) as u32);
            let mut reference = Vec::new();
            witness.rounds.execute(state, |_, row| reference.push(row)).unwrap();
            for (clock, source) in reference.iter().enumerate() {
                let row = &rows[block * CLOCKS + clock];
                assert_eq!(row.get_all_limbs(), source.state.map(split));
                assert_eq!(row.get_all_gap(), source.state.map(|v| split(MODULUS - 1 - v)));
                assert_eq!(row.get_all_quotient(), source.quotient.map(split));
                assert_eq!(row.get_all_base(), source.base);
                assert_eq!(row.get_in_use(), active.is_some());
                assert_eq!(row.get_addr(), active.map_or(0, |v| v.address));
                assert_eq!(row.get_main_step(), active.map_or(0, |v| v.step));
                for family in [row.get_all_limbs(), row.get_all_gap(), row.get_all_quotient()] {
                    for value in family.into_iter().flatten() {
                        observed[usize::from(value)] += 1;
                    }
                }
            }
            permute_packed(&mut expected).unwrap();
            let output = reference[CLOCKS - 1].state;
            let actual: [u64; 8] = std::array::from_fn(|i| {
                u64::from(output[2 * i]) | (u64::from(output[2 * i + 1]) << 32)
            });
            assert_eq!(actual, expected);
        }
        assert_eq!(counts, observed);
        assert_eq!(
            counts.iter().map(|v| u64::from(*v)).sum::<u64>(),
            (blocks * CLOCKS * 96) as u64
        );
        counts
    }

    #[test]
    fn packed_unpacked_reference_and_range_counts_match() {
        for count in [0, 1, 2, 3] {
            assert_eq!(
                check_rows::<KoalaPoseidon2TraceRow<Goldilocks>>(count, 3),
                check_rows::<KoalaPoseidon2TraceRowPacked<Goldilocks>>(count, 3)
            );
        }
    }

    #[test]
    fn complete_air_capacity_and_padding() {
        let blocks = KoalaPoseidon2Trace::<()>::NUM_ROWS / CLOCKS;
        assert_eq!(blocks, 1024);
        for count in [0, 1, blocks] {
            check_rows::<KoalaPoseidon2TraceRowPacked<Goldilocks>>(count, blocks);
        }
    }

    #[test]
    fn rejects_overflow_and_incomplete_blocks_before_writing() {
        let witness = RoundWitness::pinned();
        let mut sentinel = KoalaPoseidon2TraceRowPacked::<Goldilocks>::default();
        sentinel.set_addr(24);
        let capacity = KoalaPoseidon2Trace::<()>::NUM_ROWS / CLOCKS;
        for (length, count) in [(CLOCKS - 1, 0), (capacity * CLOCKS, capacity + 1)] {
            let mut rows = vec![sentinel; length];
            let inputs = vec![(0..count).map(input).collect::<Vec<_>>()];
            assert!(witness.fill_rows::<Goldilocks, _>(&mut rows, &inputs).is_err());
            assert!(rows.iter().all(|row| row.get_addr() == 24));
        }
    }
}
