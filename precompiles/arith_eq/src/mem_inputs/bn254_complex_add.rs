use super::ArithEqMemInputConfig;
use crate::executors::Bn254Complex;
use zisk_common::BusId;

pub const BN254_COMPLEX_ADD_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 2,
    rewrite_params: true,
    read_params: 2,
    write_params: 1,
    chunks_per_param: 8,
};

pub fn generate_bn254_complex_add_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
) -> Vec<(BusId, Vec<u64>)> {
    // op,op_type,a,b,addr[2],...
    let f1: &[u64; 8] = &data[6..14].try_into().unwrap();
    let f2: &[u64; 8] = &data[14..22].try_into().unwrap();
    let mut f3 = [0u64; 8];

    Bn254Complex::calculate_add(f1, f2, &mut f3);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&f3),
        only_counters,
        &BN254_COMPLEX_ADD_MEM_CONFIG,
    )
}
