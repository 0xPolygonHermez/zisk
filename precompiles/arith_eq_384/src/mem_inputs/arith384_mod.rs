use zisk_common::BusId;

use super::ArithEqMemInputConfig;
use crate::{executors::Arith384Mod, ARITH_EQ_384_U64S};

pub const ARITH_384_MOD_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 5,
    rewrite_params: false,
    read_params: 4,
    write_params: 1,
    chunks_per_param: ARITH_EQ_384_U64S,
};

pub fn generate_arith384_mod_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
) -> Vec<(BusId, Vec<u64>)> {
    let mut pos_offset: usize = 9; // op,op_type,a,b,addr[5],...
    let a: &[u64; ARITH_EQ_384_U64S] =
        &data[pos_offset..(pos_offset + ARITH_EQ_384_U64S)].try_into().unwrap();
    pos_offset += ARITH_EQ_384_U64S;
    let b: &[u64; ARITH_EQ_384_U64S] =
        &data[pos_offset..(pos_offset + ARITH_EQ_384_U64S)].try_into().unwrap();
    pos_offset += ARITH_EQ_384_U64S;
    let c: &[u64; ARITH_EQ_384_U64S] =
        &data[pos_offset..(pos_offset + ARITH_EQ_384_U64S)].try_into().unwrap();
    pos_offset += ARITH_EQ_384_U64S;
    let module: &[u64; ARITH_EQ_384_U64S] =
        &data[pos_offset..(pos_offset + ARITH_EQ_384_U64S)].try_into().unwrap();
    let mut d: [u64; ARITH_EQ_384_U64S] = [0u64; ARITH_EQ_384_U64S];

    Arith384Mod::calculate(a, b, c, module, &mut d);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&d),
        only_counters,
        &ARITH_384_MOD_MEM_CONFIG,
    )
}
