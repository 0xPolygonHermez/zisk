use fields::{poseidon2_hash, Goldilocks, Poseidon16, PrimeField64};
use precompiles_common::MemBusHelpers;
use precompiles_common::MemProcessor;

use zisk_common::OPERATION_BUS_DATA_SIZE;

#[derive(Debug)]
pub struct Poseidon2MemInputConfig {
    pub rewrite_params: bool,
    pub read_params: usize,
    pub write_params: usize,
    pub chunks_per_param: usize,
}

pub fn generate_poseidon2_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    mem_processors: &mut P,
) {
    // Get the basic data from the input
    // op,op_type,a,b,...
    let state: &mut [u64; 16] = &mut data[4..20].try_into().unwrap();

    // Apply the poseidon2 hash function
    let state_gl = state.map(Goldilocks::new);
    let res_gl = poseidon2_hash::<Goldilocks, Poseidon16, 16>(&state_gl);
    for (i, d) in state.iter_mut().enumerate() {
        *d = res_gl[i].as_canonical_u64();
    }

    let read_params = 1;
    let write_params = 1;
    let chunks_per_param = 16;
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
                mem_processors,
            );
        }
    }
}

pub fn skip_poseidon2_mem_inputs<P: MemProcessor>(addr_main: u32, mem_processors: &mut P) -> bool {
    let write_params = 1;
    let chunks_per_param = 16;
    for param_index in 0..write_params {
        let param_addr = addr_main + (param_index * 8 * chunks_per_param) as u32;
        for ichunk in 0..chunks_per_param {
            let addr = param_addr + ichunk as u32 * 8;
            if !mem_processors.skip_addr(addr) {
                return false;
            }
        }
    }
    true
}
