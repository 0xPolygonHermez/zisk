use precompiles_common::MemBusHelpers;
use precompiles_common::MemProcessor;

use zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE;
use zisk_core::blake2br;

use crate::blake2_constants::{
    DIRECT_READ_PARAMS, PARAMS, PARAM_CHUNKS, READ_PARAMS, START_READ_PARAMS,
};

#[derive(Debug)]
pub struct Blake2MemInputConfig {
    pub indirect_params: usize,
    pub rewrite_params: bool,
    pub read_params: usize,
    pub write_params: usize,
    pub chunks_per_param: usize,
}

pub fn generate_blake2_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    mem_processors: &mut P,
) {
    // data = [op,op_type,a,b,step,index,addr[2],state[16],input[16]]

    // Start by generating the params (direct, indirection write, indirection read)
    for iparam in 0..PARAMS {
        MemBusHelpers::mem_aligned_load(
            addr_main + iparam as u32 * 8,
            step_main,
            data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + iparam],
            mem_processors,
        );
    }

    // Generate memory load params
    for iparam in 0..READ_PARAMS {
        // let param_idx = if iparam >= DIRECT_READ_PARAM_POS { iparam + 1 } else { iparam };
        let param_idx = iparam + 1;

        let param_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + param_idx] as u32;
        for ichunk in 0..PARAM_CHUNKS {
            MemBusHelpers::mem_aligned_load(
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
        blake2br(index, &mut state, &input);
        write_data.copy_from_slice(&state);
    }

    // verify write param
    let write_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + DIRECT_READ_PARAMS] as u32;
    for (ichunk, write_data) in write_data.iter().enumerate().take(PARAM_CHUNKS) {
        let param_addr = write_addr + ichunk as u32 * 8;
        MemBusHelpers::mem_aligned_write(param_addr, step_main, *write_data, mem_processors);
    }
}

pub fn skip_blake2_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    data: &[u64],
    mem_processors: &mut P,
) -> bool {
    let indirect_params = 2;
    let read_params = 3;
    let write_params = 1;
    let chunks_per_param = [1usize, 16, 16, 16];

    for iparam in 0..indirect_params {
        let addr = addr_main + iparam as u32 * 8;
        if !mem_processors.skip_addr(addr) {
            return false;
        }
    }

    for (iparam, &chunks) in chunks_per_param.iter().enumerate().take(read_params + write_params) {
        let is_write = iparam >= read_params;
        let param_index = if is_write { iparam - read_params } else { iparam };
        let param_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + param_index] as u32;

        for ichunk in 0..chunks {
            let addr = param_addr + ichunk as u32 * 8;
            if !mem_processors.skip_addr(addr) {
                return false;
            }
        }
    }
    true
}
