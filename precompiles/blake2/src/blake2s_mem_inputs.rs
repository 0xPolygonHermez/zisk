//! Memory access pattern for the Blake2s precompile.
//!
//! Identical to Blake2b's: 3 params, two 16-word reads and one 16-word
//! write-back. Each 32-bit word occupies its own 8-byte slot with a zero
//! high half, so the addressing is unchanged.

use proofman_fields::PrimeField64;
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE;
use zisk_core::blake2sr;

use crate::blake2_constants::{
    DIRECT_READ_PARAMS, PARAMS, PARAM_CHUNKS, READ_PARAMS, START_READ_PARAMS,
};
use crate::Blake2sSM;

impl<F: PrimeField64> PrecompileMemInputs for Blake2sSM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        // data = [op,op_type,a,b,step,index,addr[2],state[16],input[16]]

        // Start by generating the params (direct, indirection write, indirection read)
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
            let param_idx = iparam + 1;
            let param_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + param_idx] as u32;
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
            let index = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE];
            let mut state: [u64; 16] =
                data[START_READ_PARAMS..START_READ_PARAMS + PARAM_CHUNKS].try_into().unwrap();
            let input: [u64; 16] = data
                [START_READ_PARAMS + PARAM_CHUNKS..START_READ_PARAMS + 2 * PARAM_CHUNKS]
                .try_into()
                .unwrap();
            blake2sr(index, &mut state, &input);
            write_data.copy_from_slice(&state);
        }

        // verify write param
        let write_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + DIRECT_READ_PARAMS] as u32;
        for (ichunk, write_data) in write_data.iter().enumerate().take(PARAM_CHUNKS) {
            let param_addr = write_addr + ichunk as u32 * 8;
            MemBusHelpers::mem_aligned_write(param_addr, step_main, *write_data, mem_processors);
        }
    }

    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processors: &mut P) -> bool {
        // Check all PARAMS words at addr_main (index, addr_state, addr_input)
        for iparam in 0..PARAMS {
            let addr = addr_main + iparam as u32 * 8;
            if !mem_processors.skip_addr(addr) {
                return false;
            }
        }

        // Check READ_PARAMS arrays (state and input, each PARAM_CHUNKS u64s)
        for iparam in 0..READ_PARAMS {
            let param_idx = iparam + 1;
            let param_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + param_idx] as u32;
            for ichunk in 0..PARAM_CHUNKS {
                let addr = param_addr + ichunk as u32 * 8;
                if !mem_processors.skip_addr(addr) {
                    return false;
                }
            }
        }

        // Check write address (output state array)
        let write_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + DIRECT_READ_PARAMS] as u32;
        for ichunk in 0..PARAM_CHUNKS {
            let addr = write_addr + ichunk as u32 * 8;
            if !mem_processors.skip_addr(addr) {
                return false;
            }
        }

        true
    }
}
