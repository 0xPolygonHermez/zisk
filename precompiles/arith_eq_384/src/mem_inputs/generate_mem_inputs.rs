use precompiles_common::MemBusHelpers;
use zisk_common::{BusId, MEM_BUS_ID, OPERATION_BUS_DATA_SIZE};

#[derive(Debug)]
pub struct ArithEqMemInputConfig {
    pub indirect_params: usize,
    pub rewrite_params: bool,
    pub read_params: usize,
    pub write_params: usize,
    pub chunks_per_param: usize,
}
pub fn generate_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    write_data: Option<&[u64]>,
    only_counters: bool,
    config: &ArithEqMemInputConfig,
) -> Vec<(BusId, Vec<u64>)> {
    let mut mem_inputs = Vec::new();
    let params_count = config.read_params + config.write_params;
    let params_offset = OPERATION_BUS_DATA_SIZE + config.indirect_params;

    for iparam in 0..config.indirect_params {
        mem_inputs.push((
            MEM_BUS_ID,
            MemBusHelpers::mem_aligned_load(
                addr_main + iparam as u32 * 8,
                step_main,
                data[OPERATION_BUS_DATA_SIZE + iparam],
            )
            .to_vec(),
        ));
    }
    for iparam in 0..params_count {
        let param_index = if config.rewrite_params && iparam >= config.read_params {
            iparam - config.read_params
        } else {
            iparam
        };
        let param_addr = if config.indirect_params > 0 {
            // read indirect parameters, means stored the address of parameter
            data[OPERATION_BUS_DATA_SIZE + param_index] as u32
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
            mem_inputs.push((
                MEM_BUS_ID,
                MemBusHelpers::mem_aligned_op(
                    param_addr + ichunk as u32 * 8,
                    step_main,
                    chunk_data,
                    is_write,
                )
                .to_vec(),
            ));
        }
    }
    mem_inputs
}
