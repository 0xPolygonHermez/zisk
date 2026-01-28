use super::ArithEqMemInputConfig;
use crate::executors::Bn254Curve;
use precompiles_common::MemProcessor;

use zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE;

pub const BN254_CURVE_ADD_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 2,
    rewrite_params: true,
    read_params: 2,
    write_params: 1,
    chunks_per_param: 8,
};

pub fn generate_bn254_curve_add_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    mem_processors: &mut P,
) {
    // op,op_type,a,b,addr[2],...
    let p1_start = OPERATION_PRECOMPILED_BUS_DATA_SIZE + BN254_CURVE_ADD_MEM_CONFIG.indirect_params;
    let p1: &[u64; 8] =
        &data[p1_start..p1_start + BN254_CURVE_ADD_MEM_CONFIG.chunks_per_param].try_into().unwrap();
    let p2_start = p1_start + BN254_CURVE_ADD_MEM_CONFIG.chunks_per_param;
    let p2: &[u64; 8] =
        &data[p2_start..p2_start + BN254_CURVE_ADD_MEM_CONFIG.chunks_per_param].try_into().unwrap();
    let mut p3 = [0u64; 8];

    Bn254Curve::calculate_add(p1, p2, &mut p3);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&p3),
        only_counters,
        mem_processors,
        &BN254_CURVE_ADD_MEM_CONFIG,
    );
}

pub fn skip_bn254_curve_add_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    data: &[u64],
    mem_processors: &mut P,
) -> bool {
    super::skip_mem_inputs(addr_main, data, &BN254_CURVE_ADD_MEM_CONFIG, mem_processors)
}
