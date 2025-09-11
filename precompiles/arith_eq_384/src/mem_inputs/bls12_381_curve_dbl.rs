use zisk_common::BusId;

use super::ArithEqMemInputConfig;
use crate::{executors::Bls12_381Curve, ARITH_EQ_384_U64S_DOUBLE};

pub const BLS12_381_CURVE_DBL_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 0,
    rewrite_params: true,
    read_params: 1,
    write_params: 1,
    chunks_per_param: ARITH_EQ_384_U64S_DOUBLE,
};

pub fn generate_bls12_381_curve_dbl_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
) -> Vec<(BusId, Vec<u64>)> {
    let pos_offset: usize = 4; // op,op_type,a,b,...
    let p1: &[u64; ARITH_EQ_384_U64S_DOUBLE] =
        &data[pos_offset..(pos_offset + ARITH_EQ_384_U64S_DOUBLE)].try_into().unwrap();
    let mut p3 = [0u64; ARITH_EQ_384_U64S_DOUBLE];

    Bls12_381Curve::calculate_dbl(p1, &mut p3);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&p3),
        only_counters,
        &BLS12_381_CURVE_DBL_MEM_CONFIG,
    )
}
