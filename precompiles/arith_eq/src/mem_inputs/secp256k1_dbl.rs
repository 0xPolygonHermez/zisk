use super::ArithEqMemInputConfig;
use crate::executors::Secp256k1;
use zisk_core::zisk_ops::swap_endianness_elements;
use zisk_precomp_common::MemProcessor;

pub const SECP256K1_DBL_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 0,
    rewrite_params: true,
    read_params: 1,
    write_params: 1,
    chunks_per_param: 8,
};

pub fn generate_secp256k1_dbl_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    big_endian: bool,
    processor: &mut P,
) {
    // op,op_type,a,b,...
    let p1: &[u64; 8] = &super::operand::<8>(&data[5..13], big_endian);
    let mut p3 = [0u64; 8];

    Secp256k1::calculate_dbl(p1, &mut p3);
    if big_endian {
        swap_endianness_elements(&mut p3, 4);
    }
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&p3),
        only_counters,
        processor,
        &SECP256K1_DBL_MEM_CONFIG,
    );
}

pub fn skip_secp256k1_dbl_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    data: &[u64],
    mem_processors: &mut P,
) -> bool {
    super::skip_mem_inputs(addr_main, data, &SECP256K1_DBL_MEM_CONFIG, mem_processors)
}
