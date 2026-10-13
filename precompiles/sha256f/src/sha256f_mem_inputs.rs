use proofman_fields::PrimeField64;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use zisk_common::{A, OPERATION_PRECOMPILED_BUS_DATA_SIZE};
use zisk_core::sha256f;

use crate::Sha256fSM;

/// Bus payload: `[op, op_type, a, b, step, state[4], input[8]]`, with a = state address (the result
/// overwrites it) and b = input address. No parameter struct, no indirections.
const STATE_WORDS: usize = 4;
const INPUT_WORDS: usize = 8;
const STATE_OFFSET: usize = OPERATION_PRECOMPILED_BUS_DATA_SIZE;
const INPUT_OFFSET: usize = STATE_OFFSET + STATE_WORDS;

impl<F: PrimeField64> PrecompileMemInputs for Sha256fSM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        let state_addr = data[A] as u32;
        let input_addr = addr_main;
        let mut state: [u64; STATE_WORDS] =
            data[STATE_OFFSET..STATE_OFFSET + STATE_WORDS].try_into().unwrap();
        let input: [u64; INPUT_WORDS] =
            data[INPUT_OFFSET..INPUT_OFFSET + INPUT_WORDS].try_into().unwrap();

        // Reads: the state at a, then the input at b
        for (i, value) in state.iter().enumerate() {
            MemBusHelpers::mem_aligned_read(
                state_addr + i as u32 * 8,
                step_main,
                *value,
                mem_processors,
            );
        }
        for (i, value) in input.iter().enumerate() {
            MemBusHelpers::mem_aligned_read(
                input_addr + i as u32 * 8,
                step_main,
                *value,
                mem_processors,
            );
        }

        // Write: the new state back at a
        if only_counters {
            state = [0; STATE_WORDS];
        } else {
            sha256f(&mut state, &input);
        }
        for (i, value) in state.iter().enumerate() {
            MemBusHelpers::mem_aligned_write(
                state_addr + i as u32 * 8,
                step_main,
                *value,
                mem_processors,
            );
        }
    }

    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processors: &mut P) -> bool {
        let state_addr = data[A] as u32;
        for i in 0..STATE_WORDS {
            if !mem_processors.skip_addr(state_addr + i as u32 * 8) {
                return false;
            }
        }
        for i in 0..INPUT_WORDS {
            if !mem_processors.skip_addr(addr_main + i as u32 * 8) {
                return false;
            }
        }
        true
    }
}
