use precompiles_common::MemProcessor;

use super::ArithEq384MemInputConfig;
use crate::{executors::Bls12_381Complex, ARITH_EQ_384_U64S_DOUBLE};

pub const BLS12_381_COMPLEX_SUB_MEM_CONFIG: ArithEq384MemInputConfig = ArithEq384MemInputConfig {
    indirect_params: 2,
    rewrite_params: true,
    read_params: 2,
    write_params: 1,
    chunks_per_param: ARITH_EQ_384_U64S_DOUBLE,
};

pub fn generate_bls12_381_complex_sub_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    mem_processors: &mut P,
) {
    let mut pos_offset: usize = 7; // op,op_type,a,b,addr[2],...
    let f1: &[u64; ARITH_EQ_384_U64S_DOUBLE] =
        &data[pos_offset..(pos_offset + ARITH_EQ_384_U64S_DOUBLE)].try_into().unwrap();
    pos_offset += ARITH_EQ_384_U64S_DOUBLE;
    let f2: &[u64; ARITH_EQ_384_U64S_DOUBLE] =
        &data[pos_offset..(pos_offset + ARITH_EQ_384_U64S_DOUBLE)].try_into().unwrap();
    let mut f3 = [0u64; ARITH_EQ_384_U64S_DOUBLE];

    Bls12_381Complex::calculate_sub(f1, f2, &mut f3);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&f3),
        only_counters,
        mem_processors,
        &BLS12_381_COMPLEX_SUB_MEM_CONFIG,
    );
}

pub fn skip_bls12_381_complex_sub_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    data: &[u64],
    mem_processors: &mut P,
) -> bool {
    super::skip_mem_inputs(addr_main, data, &BLS12_381_COMPLEX_SUB_MEM_CONFIG, mem_processors)
}
