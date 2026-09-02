use proofman_fields::PrimeField64;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE;
use zisk_core::blake3f;

use crate::blake3_constants::{PARAMS, PARAM_CHUNKS, READ_PARAMS, START_READ_PARAMS};
use crate::Blake3SM;

impl<F: PrimeField64> PrecompileMemInputs for Blake3SM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        // data = [op,op_type,a,b,step,addr[2],state[8],input[8]]

        // Start by generating the params (the two indirection reads)
        for iparam in 0..PARAMS {
            MemBusHelpers::mem_aligned_read(
                addr_main + iparam as u32 * 8,
                step_main,
                data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + iparam],
                mem_processors,
            );
        }

        // Generate memory load params
        for iparam in 0..READ_PARAMS {
            let param_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + iparam] as u32;
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
            let mut state: [u64; 8] =
                data[START_READ_PARAMS..START_READ_PARAMS + PARAM_CHUNKS].try_into().unwrap();
            let input: [u64; 8] = data
                [START_READ_PARAMS + PARAM_CHUNKS..START_READ_PARAMS + 2 * PARAM_CHUNKS]
                .try_into()
                .unwrap();
            blake3f(&mut state, &input);
            write_data.copy_from_slice(&state);
        }

        // verify write param (the permuted state goes back through the state pointer)
        let write_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE] as u32;
        for (ichunk, write_data) in write_data.iter().enumerate().take(PARAM_CHUNKS) {
            let param_addr = write_addr + ichunk as u32 * 8;
            MemBusHelpers::mem_aligned_write(param_addr, step_main, *write_data, mem_processors);
        }
    }

    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processors: &mut P) -> bool {
        // Check both param words at addr_main (addr_state, addr_input)
        for iparam in 0..PARAMS {
            let addr = addr_main + iparam as u32 * 8;
            if !mem_processors.skip_addr(addr) {
                return false;
            }
        }

        // Check READ_PARAMS arrays (state and input, each PARAM_CHUNKS u64s)
        for iparam in 0..READ_PARAMS {
            let param_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + iparam] as u32;
            for ichunk in 0..PARAM_CHUNKS {
                let addr = param_addr + ichunk as u32 * 8;
                if !mem_processors.skip_addr(addr) {
                    return false;
                }
            }
        }

        // Check write address (output state array)
        let write_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE] as u32;
        for ichunk in 0..PARAM_CHUNKS {
            let addr = write_addr + ichunk as u32 * 8;
            if !mem_processors.skip_addr(addr) {
                return false;
            }
        }

        true
    }
}
