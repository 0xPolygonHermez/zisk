use zisk_common::BusId;

use super::ArithEqMemInputConfig;
use crate::{executors::Bls12_381Curve, ARITH_EQ_384_U64S_DOUBLE};

pub const BLS12_381_CURVE_ADD_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 2,
    rewrite_params: true,
    read_params: 2,
    write_params: 1,
    chunks_per_param: 8,
};

pub fn generate_bls12_381_curve_add_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
) -> Vec<(BusId, Vec<u64>)> {
    // op,op_type,a,b,addr[2],...
    let mut pos_offset: usize = 6;
    let p1: &[u64; ARITH_EQ_384_U64S_DOUBLE] =
        &data[pos_offset..(pos_offset + ARITH_EQ_384_U64S_DOUBLE)].try_into().unwrap();
    pos_offset += ARITH_EQ_384_U64S_DOUBLE;
    let p2: &[u64; ARITH_EQ_384_U64S_DOUBLE] =
        &data[pos_offset..(pos_offset + ARITH_EQ_384_U64S_DOUBLE)].try_into().unwrap();
    let mut p3 = [0u64; ARITH_EQ_384_U64S_DOUBLE];

    Bls12_381Curve::calculate_add(p1, p2, &mut p3);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&p3),
        only_counters,
        &BLS12_381_CURVE_ADD_MEM_CONFIG,
    )
}
