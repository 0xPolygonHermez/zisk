use super::ArithEqMemInputConfig;
use crate::executors::Secp256r1;
use precompiles_common::MemProcessor;

pub const SECP256R1_DBL_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 0,
    rewrite_params: true,
    read_params: 1,
    write_params: 1,
    chunks_per_param: 8,
};

pub fn generate_secp256r1_dbl_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    processor: &mut P,
) {
    // op,op_type,a,b,...
    let p1: &[u64; 8] = &data[5..13].try_into().unwrap();
    let mut p3 = [0u64; 8];

    Secp256r1::calculate_dbl(p1, &mut p3);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&p3),
        only_counters,
        processor,
        &SECP256R1_DBL_MEM_CONFIG,
    );
}

pub fn skip_secp256r1_dbl_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    data: &[u64],
    mem_processors: &mut P,
) -> bool {
    super::skip_mem_inputs(addr_main, data, &SECP256R1_DBL_MEM_CONFIG, mem_processors)
}
