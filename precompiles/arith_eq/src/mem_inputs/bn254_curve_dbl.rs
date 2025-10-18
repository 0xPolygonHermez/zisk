use super::ArithEqMemInputConfig;
use crate::executors::Bn254Curve;
use std::collections::VecDeque;
use zisk_common::BusId;
use zisk_common::MemCollectorInfo;
use zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE;

pub const BN254_CURVE_DBL_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 0,
    rewrite_params: true,
    read_params: 1,
    write_params: 1,
    chunks_per_param: 8,
};

pub fn generate_bn254_curve_dbl_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    pending: &mut VecDeque<(BusId, Vec<u64>)>,
) {
    // op,op_type,a,b,addr[2],...
    let p1: &[u64; 8] = &data[5..13].try_into().unwrap();
    let mut p3 = [0u64; 8];

    Bn254Curve::calculate_dbl(p1, &mut p3);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&p3),
        only_counters,
        pending,
        &BN254_CURVE_DBL_MEM_CONFIG,
    );
}

pub fn skip_bn254_curve_dbl_mem_inputs(
    addr_main: u32,
    data: &[u64],
    mem_collectors_info: &[MemCollectorInfo],
) -> bool {
    super::skip_mem_inputs(addr_main, data, &BN254_CURVE_DBL_MEM_CONFIG, mem_collectors_info)
}
