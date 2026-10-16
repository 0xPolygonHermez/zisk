use proofman_fields::PrimeField64;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use zisk_common::A;
use zisk_core::blake2br;

use crate::blake2b_constants::{INDEX_POS, PARAM_CHUNKS, READ_PARAMS, START_READ_PARAMS};
use crate::Blake2bSM;

impl<F: PrimeField64> PrecompileMemInputs for Blake2bSM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        // a = state address (read and written back), b = input address (read only); see the
        // payload layout in the constants module.
        let param_addrs = [data[A] as u32, addr_main];

        // Generate memory load params
        for (iparam, param_addr) in param_addrs.iter().enumerate().take(READ_PARAMS) {
            for ichunk in 0..PARAM_CHUNKS {
                MemBusHelpers::mem_aligned_read(
                    param_addr + ichunk as u32 * 8,
                    step_main,
                    data[START_READ_PARAMS + iparam * PARAM_CHUNKS + ichunk],
                    mem_processors,
                );
            }
        }

        let mut write_data = [0u64; PARAM_CHUNKS];
        if !only_counters {
            let mut state: [u64; 16] =
                data[START_READ_PARAMS..START_READ_PARAMS + PARAM_CHUNKS].try_into().unwrap();
            let input: [u64; 16] = data
                [START_READ_PARAMS + PARAM_CHUNKS..START_READ_PARAMS + 2 * PARAM_CHUNKS]
                .try_into()
                .unwrap();
            blake2br(data[INDEX_POS], &mut state, &input);
            write_data.copy_from_slice(&state);
        }

        // verify write param (the new state goes back to the state address, a)
        let write_addr = param_addrs[0];
        for (ichunk, write_data) in write_data.iter().enumerate().take(PARAM_CHUNKS) {
            let param_addr = write_addr + ichunk as u32 * 8;
            MemBusHelpers::mem_aligned_write(param_addr, step_main, *write_data, mem_processors);
        }
    }

    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processors: &mut P) -> bool {
        // Check the READ_PARAMS arrays (state at a and input at b, each PARAM_CHUNKS u64s); the
        // state is also the write address
        for param_addr in [data[A] as u32, addr_main] {
            for ichunk in 0..PARAM_CHUNKS {
                let addr = param_addr + ichunk as u32 * 8;
                if !mem_processors.skip_addr(addr) {
                    return false;
                }
            }
        }
        true
    }
}
