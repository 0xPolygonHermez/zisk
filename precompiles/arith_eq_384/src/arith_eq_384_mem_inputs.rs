use fields::PrimeField64;
use precompiles_common::{MemProcessor, PrecompileMemInputs};
use zisk_common::OP;
use zisk_core::zisk_ops::ZiskOp;

use crate::mem_inputs::{
    generate_arith384_mod_mem_inputs, generate_bls12_381_complex_add_mem_inputs,
    generate_bls12_381_complex_mul_mem_inputs, generate_bls12_381_complex_sub_mem_inputs,
    generate_bls12_381_curve_add_mem_inputs, generate_bls12_381_curve_dbl_mem_inputs,
    skip_arith384_mod_mem_inputs, skip_bls12_381_complex_add_mem_inputs,
    skip_bls12_381_complex_mul_mem_inputs, skip_bls12_381_complex_sub_mem_inputs,
    skip_bls12_381_curve_add_mem_inputs, skip_bls12_381_curve_dbl_mem_inputs,
};
use crate::ArithEq384SM;

const ARITH384_MOD_OP: u8 = ZiskOp::Arith384Mod.code();
const BLS12_381_CURVE_ADD_OP: u8 = ZiskOp::Bls12_381CurveAdd.code();
const BLS12_381_CURVE_DBL_OP: u8 = ZiskOp::Bls12_381CurveDbl.code();
const BLS12_381_COMPLEX_ADD_OP: u8 = ZiskOp::Bls12_381ComplexAdd.code();
const BLS12_381_COMPLEX_SUB_OP: u8 = ZiskOp::Bls12_381ComplexSub.code();
const BLS12_381_COMPLEX_MUL_OP: u8 = ZiskOp::Bls12_381ComplexMul.code();

impl<F: PrimeField64> PrecompileMemInputs for ArithEq384SM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        match data[OP] as u8 {
            ARITH384_MOD_OP => generate_arith384_mod_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            BLS12_381_CURVE_ADD_OP => generate_bls12_381_curve_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            BLS12_381_CURVE_DBL_OP => generate_bls12_381_curve_dbl_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            BLS12_381_COMPLEX_ADD_OP => generate_bls12_381_complex_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            BLS12_381_COMPLEX_SUB_OP => generate_bls12_381_complex_sub_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            BLS12_381_COMPLEX_MUL_OP => generate_bls12_381_complex_mul_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            _ => panic!("ArithEq384SM::generate: unsupported sub-op {}", data[OP] as u8),
        }
    }

    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processors: &mut P) -> bool {
        match data[OP] as u8 {
            ARITH384_MOD_OP => skip_arith384_mod_mem_inputs(addr_main, data, mem_processors),
            BLS12_381_CURVE_ADD_OP => {
                skip_bls12_381_curve_add_mem_inputs(addr_main, data, mem_processors)
            }
            BLS12_381_CURVE_DBL_OP => {
                skip_bls12_381_curve_dbl_mem_inputs(addr_main, data, mem_processors)
            }
            BLS12_381_COMPLEX_ADD_OP => {
                skip_bls12_381_complex_add_mem_inputs(addr_main, data, mem_processors)
            }
            BLS12_381_COMPLEX_SUB_OP => {
                skip_bls12_381_complex_sub_mem_inputs(addr_main, data, mem_processors)
            }
            BLS12_381_COMPLEX_MUL_OP => {
                skip_bls12_381_complex_mul_mem_inputs(addr_main, data, mem_processors)
            }
            _ => panic!("ArithEq384SM::should_skip: unsupported sub-op {}", data[OP] as u8),
        }
    }
}
