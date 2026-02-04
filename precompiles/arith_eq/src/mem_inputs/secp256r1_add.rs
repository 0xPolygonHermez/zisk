use super::ArithEqMemInputConfig;
use crate::executors::Secp256r1;
use precompiles_common::MemProcessor;

pub const SECP256R1_ADD_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 2,
    rewrite_params: true,
    read_params: 2,
    write_params: 1,
    chunks_per_param: 8,
};

pub fn generate_secp256r1_add_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    mem_processors: &mut P,
) {
    // op,op_type,a,b,addr[2],...
    let p1: &[u64; 8] = &data[7..15].try_into().unwrap();
    let p2: &[u64; 8] = &data[15..23].try_into().unwrap();
    let mut p3 = [0u64; 8];

    Secp256r1::calculate_add(p1, p2, &mut p3);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&p3),
        only_counters,
        mem_processors,
        &SECP256R1_ADD_MEM_CONFIG,
    );
}

pub fn skip_secp256r1_add_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    data: &[u64],
    mem_processors: &mut P,
) -> bool {
    super::skip_mem_inputs(addr_main, data, &SECP256R1_ADD_MEM_CONFIG, mem_processors)
}
