use tiny_keccak::keccakf;

use precompiles_common::MemBusHelpers;
use std::collections::VecDeque;
use zisk_common::MemCollectorInfo;
use zisk_common::{BusId, OPERATION_BUS_DATA_SIZE};

#[derive(Debug)]
pub struct KeccakfMemInputConfig {
    pub rewrite_params: bool,
    pub read_params: usize,
    pub write_params: usize,
    pub chunks_per_param: usize,
}

pub fn generate_keccakf_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    pending: &mut VecDeque<(BusId, Vec<u64>)>,
) {
    // Get the basic data from the input
    // op,op_type,a,b,...
    let state: &mut [u64; 25] = &mut data[4..29].try_into().unwrap();

    // Apply the keccakf function
    keccakf(state);

    // Generate the memory reads/writes
    let read_params = 1;
    let write_params = 1;
    let chunks_per_param = 25;
    let params_count = read_params + write_params;
    let params_offset = OPERATION_BUS_DATA_SIZE;
    for iparam in 0..params_count {
        let is_write = iparam >= read_params;
        let param_index = if is_write { iparam - read_params } else { iparam };
        let param_addr = addr_main + (param_index * 8 * chunks_per_param) as u32;

        // read/write all chunks of the iparam parameter
        let current_param_offset = if is_write {
            // if write calculate index over write_data
            chunks_per_param * (iparam - read_params)
        } else {
            params_offset + chunks_per_param * iparam
        };
        for ichunk in 0..chunks_per_param {
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

pub fn skip_keccakf_mem_inputs(addr_main: u32, mem_collectors_info: &[MemCollectorInfo]) -> bool {
    let write_params = 1;
    let chunks_per_param = 25;
    for param_index in 0..write_params {
        let param_addr = addr_main + (param_index * 8 * chunks_per_param) as u32;
        for ichunk in 0..chunks_per_param {
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
