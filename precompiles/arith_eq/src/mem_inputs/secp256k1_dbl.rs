use super::ArithEqMemInputConfig;
use crate::executors::Secp256k1;
use zisk_common::BusId;

pub const SECP256K1_DBL_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 0,
    rewrite_params: true,
    read_params: 1,
    write_params: 1,
    chunks_per_param: 8,
};

pub fn generate_secp256k1_dbl_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
) -> Vec<(BusId, Vec<u64>)> {
    // op,op_type,a,b,addr[2],...
    let p1: &[u64; 8] = &data[4..12].try_into().unwrap();
    let mut p3 = [0u64; 8];

    Secp256k1::calculate_dbl(p1, &mut p3);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&p3),
        only_counters,
        &SECP256K1_DBL_MEM_CONFIG,
    )
}
