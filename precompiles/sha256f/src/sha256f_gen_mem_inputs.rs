use precompiles_common::MemBusHelpers;
use precompiles_common::MemProcessor;

use zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE;
use zisk_core::sha256f;

#[derive(Debug)]
pub struct Sha256MemInputConfig {
    pub indirect_params: usize,
    pub rewrite_params: bool,
    pub read_params: usize,
    pub write_params: usize,
    pub chunks_per_param: usize,
}

pub fn generate_sha256f_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    mem_processors: &mut P,
) {
    // Get the basic data from the input
    // op,op_type,a,b,addr[2],...
    let state: &mut [u64; 4] = &mut data[7..11].try_into().unwrap();
    let input: &[u64; 8] = &data[11..19].try_into().unwrap();

    // Apply the sha256f function and get the output
    sha256f(state, input);

    // Generate the memory reads/writes
    let indirect_params = 2;

    // Start by generating the indirection reads
    for iparam in 0..indirect_params {
        MemBusHelpers::mem_aligned_load(
            addr_main + iparam as u32 * 8,
            step_main,
            data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + iparam],
            mem_processors,
        );
    }

    // Now we can treat the raw inputs
    let read_params = 2;
    let write_params = 1;
    let chunks_per_param = [4usize, 8, 4];
    let params_count = read_params + write_params;
    let params_offset = OPERATION_PRECOMPILED_BUS_DATA_SIZE + indirect_params;
    let mut read_chunks = 0;
    for (iparam, &chunks) in chunks_per_param.iter().enumerate().take(params_count) {
        let is_write = iparam >= read_params;
        let param_index = if is_write { iparam - read_params } else { iparam };
        let param_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + param_index] as u32;
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
                mem_processors,
            );
        }
    }
}

pub fn skip_sha256f_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    data: &[u64],
    mem_processors: &mut P,
) -> bool {
    let indirect_params = 2;
    let read_params = 2;
    let write_params = 1;
    let chunks_per_param = [4usize, 8, 4];

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
