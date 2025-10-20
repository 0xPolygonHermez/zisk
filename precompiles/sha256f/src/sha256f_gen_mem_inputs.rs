use sha2::compress256;

use precompiles_common::MemBusHelpers;
use std::collections::VecDeque;
use zisk_common::MemCollectorInfo;
use zisk_common::{BusId, OPERATION_BUS_DATA_SIZE};
use zisk_core::{convert_u32_to_u64, convert_u64_to_generic_array_bytes, convert_u64_to_u32};

#[derive(Debug)]
pub struct Sha256MemInputConfig {
    pub indirect_params: usize,
    pub rewrite_params: bool,
    pub read_params: usize,
    pub write_params: usize,
    pub chunks_per_param: usize,
}

pub fn generate_sha256f_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    pending: &mut VecDeque<(BusId, Vec<u64>)>,
) {
    // Get the basic data from the input
    // op,op_type,a,b,addr[2],...
    let state: &mut [u64; 4] = &mut data[6..10].try_into().unwrap();
    let input: &[u64; 8] = &data[10..18].try_into().unwrap();

    // Apply the sha256f function and get the output
    let mut state_u32: [u32; 8] = convert_u64_to_u32(state).try_into().unwrap();
    let block = convert_u64_to_generic_array_bytes(input);
    compress256(&mut state_u32, &[block]);

    *state = convert_u32_to_u64(&state_u32);

    // Generate the memory reads/writes
    let indirect_params = 2;

    // Start by generating the indirection reads
    for iparam in 0..indirect_params {
        MemBusHelpers::mem_aligned_load(
            addr_main + iparam as u32 * 8,
            step_main,
            data[OPERATION_BUS_DATA_SIZE + iparam],
            pending,
        );
    }

    // Now we can treat the raw inputs
    let read_params = 2;
    let write_params = 1;
    let chunks_per_param = [4usize, 8, 4];
    let params_count = read_params + write_params;
    let params_offset = OPERATION_BUS_DATA_SIZE + indirect_params;
    let mut read_chunks = 0;
    for (iparam, &chunks) in chunks_per_param.iter().enumerate().take(params_count) {
        let is_write = iparam >= read_params;
        let param_index = if is_write { iparam - read_params } else { iparam };
        let param_addr = data[OPERATION_BUS_DATA_SIZE + param_index] as u32;
        // read/write all chunks of the iparam parameter
        let current_param_offset = if is_write {
            // if write calculate index over write_data
            chunks * param_index
        } else {
            // if read calculate param
            let offset = params_offset + read_chunks;
            read_chunks += chunks;
            offset
        };
        for ichunk in 0..chunks {
            let chunk_data = if only_counters && is_write {
                0
            } else if is_write {
                state[current_param_offset + ichunk]
            } else {
                data[current_param_offset + ichunk]
            };
            MemBusHelpers::mem_aligned_op(
                param_addr + ichunk as u32 * 8,
                step_main,
                chunk_data,
                is_write,
                pending,
            );
        }
    }
}

pub fn skip_sha256f_mem_inputs(
    addr_main: u32,
    data: &[u64],
    mem_collectors_info: &[MemCollectorInfo],
) -> bool {
    let indirect_params = 2;
    let read_params = 2;
    let write_params = 1;
    let chunks_per_param = [4usize, 8, 4];

    for iparam in 0..indirect_params {
        let addr = addr_main + iparam as u32 * 8;
        for mem_collector in mem_collectors_info {
            if !mem_collector.skip_addr(addr) {
                return false;
            }
        }
    }

    for (iparam, &chunks) in chunks_per_param.iter().enumerate().take(read_params + write_params) {
        let is_write = iparam >= read_params;
        let param_index = if is_write { iparam - read_params } else { iparam };
        let param_addr = data[OPERATION_BUS_DATA_SIZE + param_index] as u32;

        for ichunk in 0..chunks {
            let addr = param_addr + ichunk as u32 * 8;
            for mem_collector in mem_collectors_info {
                if !mem_collector.skip_addr(addr) {
                    return false;
                }
            }
        }
    }
    true
}
