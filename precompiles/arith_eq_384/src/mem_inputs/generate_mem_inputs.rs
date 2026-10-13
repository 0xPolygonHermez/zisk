use zisk_common::{A, OPERATION_PRECOMPILED_BUS_DATA_SIZE};
use zisk_precomp_common::MemBusHelpers;
use zisk_precomp_common::MemProcessor;

#[derive(Debug)]
pub struct ArithEq384MemInputConfig {
    /// Number of parameter addresses read from the parameter struct pointed to by `b`.
    pub indirect_params: usize,
    /// The op is called with two direct operands: the first parameter lives at `a`, the second at
    /// `b`, and there is no parameter struct (`indirect_params` must be 0).
    pub direct_ab: bool,
    pub rewrite_params: bool,
    pub read_params: usize,
    pub write_params: usize,
    pub chunks_per_param: usize,
}
/// `addr_main` is the main operand b (the parameter struct address, or the second operand of a
/// direct a/b op).
pub fn generate_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    write_data: Option<&[u64]>,
    only_counters: bool,
    mem_processors: &mut P,
    config: &ArithEq384MemInputConfig,
) {
    let params_count = config.read_params + config.write_params;
    let params_offset = OPERATION_PRECOMPILED_BUS_DATA_SIZE + config.indirect_params;

    for iparam in 0..config.indirect_params {
        MemBusHelpers::mem_aligned_read(
            addr_main + iparam as u32 * 8,
            step_main,
            data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + iparam],
            mem_processors,
        )
    }
    for iparam in 0..params_count {
        let param_index = if config.rewrite_params && iparam >= config.read_params {
            iparam - config.read_params
        } else {
            iparam
        };
        let param_addr = if config.indirect_params > 0 {
            // read indirect parameters, means stored the address of parameter
            data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + param_index] as u32
        } else if config.direct_ab {
            // first parameter at a, second at b
            if param_index == 0 {
                data[A] as u32
            } else {
                addr_main
            }
        } else {
            addr_main + (param_index * 8 * config.chunks_per_param) as u32
        };

        // read/write all chunks of the iparam parameter
        let is_write = iparam >= config.read_params;
        let current_param_offset = if is_write {
            // if write calculate index over write_data
            config.chunks_per_param * (iparam - config.read_params)
        } else {
            // if read calculate param
            params_offset + config.chunks_per_param * iparam
        };
        for ichunk in 0..config.chunks_per_param {
            let chunk_data = if only_counters && is_write {
                0
            } else if is_write {
                write_data.unwrap()[current_param_offset + ichunk]
            } else {
                data[current_param_offset + ichunk]
            };
            MemBusHelpers::mem_aligned_op(
                param_addr + ichunk as u32 * 8,
                step_main,
                chunk_data,
                is_write,
                mem_processors,
            )
        }
    }
}

pub fn skip_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    data: &[u64],
    config: &ArithEq384MemInputConfig,
    mem_processors: &mut P,
) -> bool {
    let params_count = config.read_params + config.write_params;

    // Check indirect loads
    for iparam in 0..config.indirect_params {
        let addr = addr_main + iparam as u32 * 8;
        if !mem_processors.skip_addr(addr) {
            return false;
        }
    }

    for iparam in 0..params_count {
        let param_index = if config.rewrite_params && iparam >= config.read_params {
            iparam - config.read_params
        } else {
            iparam
        };
        let param_addr = if config.indirect_params > 0 {
            data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + param_index] as u32
        } else if config.direct_ab {
            if param_index == 0 {
                data[A] as u32
            } else {
                addr_main
            }
        } else {
            addr_main + (param_index * 8 * config.chunks_per_param) as u32
        };
        for ichunk in 0..config.chunks_per_param {
            let addr = param_addr + ichunk as u32 * 8;
            if !mem_processors.skip_addr(addr) {
                return false;
            }
        }
    }
    true
}
