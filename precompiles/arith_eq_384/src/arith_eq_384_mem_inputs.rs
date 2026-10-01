use proofman_fields::PrimeField64;
use zisk_common::OP;
use zisk_core::zisk_ops::ZiskOp;

use crate::arith_eq_384_op_is_big_endian;
use zisk_precomp_common::{MemProcessor, PrecompileMemInputs};

use crate::mem_inputs::{
    generate_arith384_mod_mem_inputs, generate_bls12_381_complex_add_mem_inputs,
    generate_bls12_381_complex_mul_mem_inputs, generate_bls12_381_complex_sub_mem_inputs,
    generate_bls12_381_curve_add_mem_inputs, generate_bls12_381_curve_dbl_mem_inputs,
    skip_arith384_mod_mem_inputs, skip_bls12_381_complex_add_mem_inputs,
    skip_bls12_381_complex_mul_mem_inputs, skip_bls12_381_complex_sub_mem_inputs,
    skip_bls12_381_curve_add_mem_inputs, skip_bls12_381_curve_dbl_mem_inputs,
};
use crate::ArithEq384SM;

impl<F: PrimeField64> PrecompileMemInputs for ArithEq384SM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        // A big-endian op is the same memory access pattern as its little-endian twin; only the
        // operand values need converting, which the generators do from `big_endian`.
        let op = data[OP] as u8;
        let big_endian = arith_eq_384_op_is_big_endian(op)
            .unwrap_or_else(|| panic!("ArithEq384SM::generate: unsupported sub-op {op}"));
        match op | 0x80 {
            ZiskOp::ARITH384_MOD => generate_arith384_mod_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ZiskOp::BLS12_381_CURVE_ADD => generate_bls12_381_curve_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ZiskOp::BLS12_381_CURVE_DBL => generate_bls12_381_curve_dbl_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ZiskOp::BLS12_381_COMPLEX_ADD => generate_bls12_381_complex_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ZiskOp::BLS12_381_COMPLEX_SUB => generate_bls12_381_complex_sub_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ZiskOp::BLS12_381_COMPLEX_MUL => generate_bls12_381_complex_mul_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            _ => unreachable!("checked above"),
        }
    }

    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processors: &mut P) -> bool {
        let op = data[OP] as u8;
        if arith_eq_384_op_is_big_endian(op).is_none() {
            panic!("ArithEq384SM::should_skip: unsupported sub-op {op}");
        }
        // The big-endian twin has the little-endian opcode with bit 7 cleared.
        match op | 0x80 {
            ZiskOp::ARITH384_MOD => skip_arith384_mod_mem_inputs(addr_main, data, mem_processors),
            ZiskOp::BLS12_381_CURVE_ADD => {
                skip_bls12_381_curve_add_mem_inputs(addr_main, data, mem_processors)
            }
            ZiskOp::BLS12_381_CURVE_DBL => {
                skip_bls12_381_curve_dbl_mem_inputs(addr_main, data, mem_processors)
            }
            ZiskOp::BLS12_381_COMPLEX_ADD => {
                skip_bls12_381_complex_add_mem_inputs(addr_main, data, mem_processors)
            }
            ZiskOp::BLS12_381_COMPLEX_SUB => {
                skip_bls12_381_complex_sub_mem_inputs(addr_main, data, mem_processors)
            }
            ZiskOp::BLS12_381_COMPLEX_MUL => {
                skip_bls12_381_complex_mul_mem_inputs(addr_main, data, mem_processors)
            }
            _ => unreachable!("checked above"),
        }
    }
}
