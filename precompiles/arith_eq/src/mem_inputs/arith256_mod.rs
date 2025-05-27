use super::ArithEqMemInputConfig;
use crate::executors::Arith256Mod;
use zisk_common::BusId;

pub const ARITH_256_MOD_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 5,
    rewrite_params: false,
    read_params: 4,
    write_params: 1,
    chunks_per_param: 4,
};
pub fn generate_arith256_mod_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
) -> Vec<(BusId, Vec<u64>)> {
    // op,op_type,a,b,addr[5],...
    let a: &[u64; 4] = &data[9..13].try_into().unwrap();
    let b: &[u64; 4] = &data[13..17].try_into().unwrap();
    let c: &[u64; 4] = &data[17..21].try_into().unwrap();
    let module: &[u64; 4] = &data[21..25].try_into().unwrap();
    let mut d: [u64; 4] = [0u64; 4];

    Arith256Mod::calculate(a, b, c, module, &mut d);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&d),
        only_counters,
        &ARITH_256_MOD_MEM_CONFIG,
    )
}
