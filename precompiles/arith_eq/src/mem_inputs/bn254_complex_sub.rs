use super::ArithEqMemInputConfig;
use crate::executors::Bn254Complex;
use std::collections::VecDeque;
use zisk_common::BusId;
use zisk_common::MemCollectorInfo;

pub const BN254_COMPLEX_SUB_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 2,
    rewrite_params: true,
    read_params: 2,
    write_params: 1,
    chunks_per_param: 8,
};

pub fn generate_bn254_complex_sub_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    pending: &mut VecDeque<(BusId, Vec<u64>)>,
) {
    // op,op_type,a,b,addr[2],...
    let f1: &[u64; 8] = &data[7..15].try_into().unwrap();
    let f2: &[u64; 8] = &data[15..23].try_into().unwrap();
    let mut f3 = [0u64; 8];

    Bn254Complex::calculate_sub(f1, f2, &mut f3);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&f3),
        only_counters,
        pending,
        &BN254_COMPLEX_SUB_MEM_CONFIG,
    );
}

pub fn skip_bn254_complex_sub_mem_inputs(
    addr_main: u32,
    data: &[u64],
    mem_collectors_info: &[MemCollectorInfo],
) -> bool {
    super::skip_mem_inputs(addr_main, data, &BN254_COMPLEX_SUB_MEM_CONFIG, mem_collectors_info)
}
