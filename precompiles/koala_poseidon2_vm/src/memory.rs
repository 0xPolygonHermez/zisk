//! Memory-bus tuples for one call: eight aligned reads of the input words, then eight
//! writes of the output words at the same addresses. Steps follow the precompile
//! convention (reads at `4 * step + 3`, writes at `4 * step + 4`), matching the AIR.

use proofman_fields::PrimeField64;
use zisk_definitions::koala_poseidon2::permute_packed;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use crate::{KoalaPoseidon2Input, KoalaPoseidon2SM};

/// Decodes the payload and checks it against the address the executor passed alongside.
fn checked_input(address: u32, data: &[u64]) -> KoalaPoseidon2Input {
    let input = KoalaPoseidon2Input::from(data.try_into().expect("invalid KoalaPoseidon2 payload"));
    assert_eq!(address, input.address, "KoalaPoseidon2 address mismatch");
    input
}

impl<F: PrimeField64> PrecompileMemInputs for KoalaPoseidon2SM<F> {
    fn generate<P: MemProcessor>(
        address: u32,
        step: u64,
        data: &[u64],
        only_counters: bool,
        processors: &mut P,
    ) {
        let input = checked_input(address, data);
        assert_eq!(step, input.step, "KoalaPoseidon2 step mismatch");
        // Counting only needs the addresses and steps; skip the permutation then.
        let mut output = input.state;
        if !only_counters {
            permute_packed(&mut output).expect("noncanonical KoalaPoseidon2 input");
        }
        for (write, words) in [(false, input.state), (true, output)] {
            for (index, word) in words.into_iter().enumerate() {
                let value = if only_counters && write { 0 } else { word };
                MemBusHelpers::mem_aligned_op(
                    address + 8 * index as u32,
                    step,
                    value,
                    write,
                    processors,
                );
            }
        }
    }

    fn should_skip<P: MemProcessor>(address: u32, data: &[u64], processors: &mut P) -> bool {
        checked_input(address, data);
        (0..8).all(|index| processors.skip_addr(address + 8 * index))
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use proofman_fields::Goldilocks;
    use zisk_common::OperationKoalaPoseidon2Data;
    use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
    use zisk_definitions::koala_poseidon2::MODULUS;

    use super::*;

    type Sm = KoalaPoseidon2SM<Goldilocks>;

    #[derive(Default)]
    struct Recorder {
        events: Vec<[u64; 7]>,
        skipped: Vec<u32>,
        needed: Option<u32>,
    }

    impl MemProcessor for Recorder {
        fn process_mem_data(&mut self, data: &[u64; 7]) {
            self.events.push(*data);
        }

        fn skip_addr(&mut self, address: u32) -> bool {
            self.skipped.push(address);
            self.needed != Some(address)
        }

        fn skip_addr_range(&mut self, _from: u32, _to: u32) -> bool {
            panic!("unexpected range query")
        }
    }

    fn payload(address: u32, step: u64) -> OperationKoalaPoseidon2Data<u64> {
        let mut data = [0; 13];
        data[..5].copy_from_slice(&[
            ZiskOp::KOALA_POSEIDON2 as u64,
            ZiskOperationType::KoalaPoseidon2 as u64,
            0,
            u64::from(address),
            step,
        ]);
        for (index, word) in data[5..].iter_mut().enumerate() {
            *word = index as u64 | (u64::from(MODULUS - 1 - index as u32) << 32);
        }
        data
    }

    #[test]
    fn emits_all_eight_reads_then_eight_writes_with_exact_timestamps() {
        for (address, step) in [(0, 0), (0xa000_1000, 17), (0xffff_ffc0, (1 << 40) - 1)] {
            let data = payload(address, step);
            let mut expected: [u64; 8] = data[5..].try_into().unwrap();
            permute_packed(&mut expected).unwrap();
            for only_counters in [false, true] {
                let mut recorder = Recorder::default();
                Sm::generate(address, step, &data, only_counters, &mut recorder);
                assert_eq!(recorder.events.len(), 16);
                for index in 0..8 {
                    let current_address = u64::from(address) + 8 * index as u64;
                    assert_eq!(
                        recorder.events[index],
                        [1, current_address, 4 * step + 3, 8, data[5 + index], 0, 0]
                    );
                    assert_eq!(
                        recorder.events[index + 8],
                        [
                            2,
                            current_address,
                            4 * step + 4,
                            8,
                            0,
                            0,
                            if only_counters { 0 } else { expected[index] }
                        ]
                    );
                }
            }
        }
    }

    #[test]
    fn consecutive_in_place_calls_read_the_previous_output() {
        let address = 0xa000_1000;
        let mut data = payload(address, 19);
        let mut recorder = Recorder::default();
        Sm::generate(address, 19, &data, false, &mut recorder);
        let output: [u64; 8] = std::array::from_fn(|index| recorder.events[8 + index][6]);
        data[5..].copy_from_slice(&output);
        data[4] = 20;
        Sm::generate(address, 20, &data, false, &mut recorder);
        assert_eq!(recorder.events.len(), 32);
        for index in 0..8 {
            assert_eq!(recorder.events[16 + index][4], recorder.events[8 + index][6]);
            assert!(recorder.events[16 + index][2] > recorder.events[8 + index][2]);
        }
    }

    fn assert_rejected(address: u32, step: u64, data: &[u64], reject_skip: bool) {
        for only_counters in [false, true] {
            let mut recorder = Recorder::default();
            assert!(catch_unwind(AssertUnwindSafe(|| {
                Sm::generate(address, step, data, only_counters, &mut recorder);
            }))
            .is_err());
            assert!(recorder.events.is_empty());
        }
        if reject_skip {
            let mut recorder = Recorder::default();
            assert!(catch_unwind(AssertUnwindSafe(|| {
                Sm::should_skip(address, data, &mut recorder);
            }))
            .is_err());
            assert!(recorder.skipped.is_empty());
            assert!(recorder.events.is_empty());
        }
    }

    #[test]
    fn rejects_invalid_headers_addresses_and_steps_before_emitting_memory() {
        let address = 0xa000_1000;
        let step = 17;
        let data = payload(address, step);
        for (index, value) in [
            (0, ZiskOp::POSEIDON1 as u64),
            (0, u64::from(ZiskOp::KOALA_POSEIDON2) + 256),
            (1, ZiskOperationType::Poseidon as u64),
            (2, 1),
            (3, u64::from(address) + (1 << 32)),
            (4, 1 << 40),
        ] {
            let mut invalid = data;
            invalid[index] = value;
            assert_rejected(address, step, &invalid, true);
        }
        assert_rejected(address + 8, step, &data, true);
        assert_rejected(address, step + 1, &data, false);
        for invalid_address in [address + 1, 0xffff_fff8] {
            assert_rejected(invalid_address, step, &payload(invalid_address, step), true);
        }
        for length in 0..13 {
            assert_rejected(address, step, &data[..length], true);
        }
        let mut oversized = data.to_vec();
        oversized.push(0);
        assert_rejected(address, step, &oversized, true);
    }

    #[test]
    fn rejects_each_noncanonical_lane_before_emitting_memory() {
        let address = 0xa000_1000;
        let step = 17;
        for lane in 0..16 {
            for value in [MODULUS, u32::MAX] {
                let mut data = payload(address, step);
                let shift = 32 * (lane % 2);
                data[5 + lane / 2] &= !(u64::from(u32::MAX) << shift);
                data[5 + lane / 2] |= u64::from(value) << shift;
                assert_rejected(address, step, &data, true);
            }
        }
    }

    #[test]
    fn skips_only_when_every_input_and_output_word_is_unneeded() {
        let address = 0xa000_1000;
        let data = payload(address, 17);
        let mut recorder = Recorder::default();
        assert!(Sm::should_skip(address, &data, &mut recorder));
        assert_eq!(recorder.skipped, (0..8).map(|index| address + 8 * index).collect::<Vec<_>>());
        for index in 0..8 {
            let mut recorder = Recorder { needed: Some(address + 8 * index), ..Default::default() };
            assert!(!Sm::should_skip(address, &data, &mut recorder));
            assert!(recorder.events.is_empty());
        }
    }
}
