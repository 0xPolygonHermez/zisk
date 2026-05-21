use fields::PrimeField64;
use lib_c::add256;
use precompiles_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};

use zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE;

use crate::add256_constants::*;
use crate::Add256SM;

impl<F: PrimeField64> PrecompileMemInputs for Add256SM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        // Start by generating the params (indirection read, direct, indirection write)
        for iparam in 0..PARAMS {
            MemBusHelpers::mem_aligned_read(
                addr_main + iparam as u32 * 8,
                step_main,
                data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + iparam],
                mem_processors,
            );
        }

        // generate load params
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
            let a: [u64; 4] =
                data[START_READ_PARAMS..START_READ_PARAMS + PARAM_CHUNKS].try_into().unwrap();
            let b: [u64; 4] = data
                [START_READ_PARAMS + PARAM_CHUNKS..START_READ_PARAMS + 2 * PARAM_CHUNKS]
                .try_into()
                .unwrap();
            add256(
                &a,
                &b,
                data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + READ_PARAMS],
                &mut write_data,
            );
        }

        // verify write param
        let write_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + WRITE_ADDR_PARAM] as u32;
        for (ichunk, write_data) in write_data.iter().enumerate().take(PARAM_CHUNKS) {
            let param_addr = write_addr + ichunk as u32 * 8;
            MemBusHelpers::mem_aligned_write(param_addr, step_main, *write_data, mem_processors);
        }
    }

    // op_a = step
    // op_b = addr_main
    // mem_trace: @a, @b, cin, @c, a[0..3], b[0..3], cout, [ c[0..3] ]
    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processors: &mut P) -> bool {
        // verify main params "struct" of indirections
        for iparam in 0..PARAMS {
            let addr = addr_main + iparam as u32 * 8;
            if !mem_processors.skip_addr(addr) {
                return false;
            }
        }

        // verify read params
        for iparam in 0..READ_PARAMS {
            let param_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + iparam] as u32;
            for ichunk in 0..PARAM_CHUNKS {
                let addr = param_addr + ichunk as u32 * 8;
                if !mem_processors.skip_addr(addr) {
                    return false;
                }
            }
        }

        // verify write param
        let write_addr = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + WRITE_ADDR_PARAM] as u32;
        for ichunk in 0..PARAM_CHUNKS {
            let addr = write_addr + ichunk as u32 * 8;
            if !mem_processors.skip_addr(addr) {
                return false;
            }
        }

        true
    }
}
