use proofman_fields::PrimeField64;
use tiny_keccak::keccakf;
use zisk_core::KECCAKF_XOR_LANES;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use crate::{
    keccakf_addresses, keccakf_is_xor, KeccakfSM, KECCAKF_PAYLOAD_BLOCK, KECCAKF_PAYLOAD_STATE,
};

const STATE_WORDS: usize = 25;

impl<F: PrimeField64> PrecompileMemInputs for KeccakfSM<F> {
    /// Memory accesses of a Keccak-f invocation: the 25 words of the state are read at the main
    /// step and written, with the output of the permutation, at the next one. A `keccak_xor`
    /// also reads the 17 words of its block at the main step.
    ///
    /// `_addr_main` is not used: which operand holds the state depends on the operation.
    fn generate<P: MemProcessor>(
        _addr_main: u64,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        let (state_addr, block_addr) = keccakf_addresses(data);
        let is_xor = keccakf_is_xor(data);

        // Read the state as it is in memory
        for (i, value) in data[KECCAKF_PAYLOAD_STATE..KECCAKF_PAYLOAD_BLOCK].iter().enumerate() {
            MemBusHelpers::mem_aligned_op(
                state_addr + i as u64 * 8,
                step_main,
                *value,
                false,
                mem_processors,
            );
        }

        // Read the block
        if is_xor {
            let block = &data[KECCAKF_PAYLOAD_BLOCK..KECCAKF_PAYLOAD_BLOCK + KECCAKF_XOR_LANES];
            for (i, value) in block.iter().enumerate() {
                MemBusHelpers::mem_aligned_op(
                    block_addr + i as u64 * 8,
                    step_main,
                    *value,
                    false,
                    mem_processors,
                );
            }
        }

        // Write the output of the permutation. The counters only need the accesses, not the values
        let mut state = [0u64; STATE_WORDS];
        if !only_counters {
            state.copy_from_slice(&data[KECCAKF_PAYLOAD_STATE..KECCAKF_PAYLOAD_BLOCK]);
            if is_xor {
                let block = &data[KECCAKF_PAYLOAD_BLOCK..KECCAKF_PAYLOAD_BLOCK + KECCAKF_XOR_LANES];
                for (lane, word) in state.iter_mut().zip(block) {
                    *lane ^= *word;
                }
            }
            keccakf(&mut state);
        }
        for (i, value) in state.iter().enumerate() {
            MemBusHelpers::mem_aligned_op(
                state_addr + i as u64 * 8,
                step_main,
                *value,
                true,
                mem_processors,
            );
        }
    }

    /// The invocation can be skipped when no collector wants any of the words it touches: the
    /// state and, for a `keccak_xor`, the block.
    fn should_skip<P: MemProcessor>(_addr_main: u64, data: &[u64], mem_processors: &mut P) -> bool {
        let (state_addr, block_addr) = keccakf_addresses(data);
        if !(0..STATE_WORDS).all(|i| mem_processors.skip_addr(state_addr + i as u64 * 8)) {
            return false;
        }
        !keccakf_is_xor(data)
            || (0..KECCAKF_XOR_LANES).all(|i| mem_processors.skip_addr(block_addr + i as u64 * 8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_common::{A, B, OP, STEP};
    use zisk_core::zisk_ops::ZiskOp;

    use crate::KECCAKF_PAYLOAD_DEDUP_WORD;

    /// Records the (is_write, address, value) of every access.
    #[derive(Default)]
    struct Recorder(Vec<(bool, u64, u64)>);

    impl MemProcessor for Recorder {
        fn process_mem_data(&mut self, data: &[u64; 7]) {
            let is_write = data[0] == 2; // MEMORY_STORE_OP
            self.0.push((is_write, data[1], if is_write { data[6] } else { data[4] }));
        }
        fn skip_addr(&mut self, _addr: u64) -> bool {
            false
        }
        fn skip_addr_range(&mut self, _from: u64, _to: u64) -> bool {
            false
        }
    }

    /// The dedup word follows the block in the payload and must not leak into the XOR: with a
    /// non-zero word, a XOR that ran one word too far would change lane 17 of what is written.
    #[test]
    fn keccak_xor_reads_state_and_block_and_writes_the_permuted_xor() {
        let state: [u64; 25] = core::array::from_fn(|i| 0x0101_0101_0101_0101 * (i as u64 + 1));
        let block: [u64; KECCAKF_XOR_LANES] = core::array::from_fn(|i| 0xa5a5 + i as u64);

        let mut data = vec![0u64; KECCAKF_PAYLOAD_DEDUP_WORD + 1];
        data[OP] = ZiskOp::KeccakXor.code() as u64;
        data[A] = 0xc100_0000;
        data[B] = 0xc200_0000;
        data[STEP] = 9;
        data[KECCAKF_PAYLOAD_STATE..KECCAKF_PAYLOAD_BLOCK].copy_from_slice(&state);
        data[KECCAKF_PAYLOAD_BLOCK..KECCAKF_PAYLOAD_DEDUP_WORD].copy_from_slice(&block);
        data[KECCAKF_PAYLOAD_DEDUP_WORD] = 0x8000_0000_0000_0123;

        let mut recorder = Recorder::default();
        <KeccakfSM<Goldilocks> as PrecompileMemInputs>::generate(0, 9, &data, false, &mut recorder);

        let mut expected = state;
        for (lane, word) in expected.iter_mut().zip(&block) {
            *lane ^= *word;
        }
        keccakf(&mut expected);

        let accesses = recorder.0;
        assert_eq!(accesses.len(), 25 + KECCAKF_XOR_LANES + 25);
        for i in 0..25 {
            assert_eq!(accesses[i], (false, 0xc100_0000 + 8 * i as u64, state[i]));
            let write = accesses[25 + KECCAKF_XOR_LANES + i];
            assert_eq!((write.1, write.2), (0xc100_0000 + 8 * i as u64, expected[i]));
        }
        for i in 0..KECCAKF_XOR_LANES {
            assert_eq!(accesses[25 + i], (false, 0xc200_0000 + 8 * i as u64, block[i]));
        }
    }
}
