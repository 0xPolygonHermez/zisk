use proofman_fields::PrimeField64;
use zisk_common::OP;
use zisk_precomp_common::{MemProcessor, PrecompileMemInputs};

use crate::mem_inputs::{
    generate_arith256_mem_inputs, generate_arith256_mod_mem_inputs,
    generate_bn254_complex_add_mem_inputs, generate_bn254_complex_mul_mem_inputs,
    generate_bn254_complex_sub_mem_inputs, generate_bn254_curve_add_mem_inputs,
    generate_bn254_curve_dbl_mem_inputs, generate_secp256k1_add_mem_inputs,
    generate_secp256k1_dbl_mem_inputs, generate_secp256r1_add_mem_inputs,
    generate_secp256r1_dbl_mem_inputs, skip_arith256_mem_inputs, skip_arith256_mod_mem_inputs,
    skip_bn254_complex_add_mem_inputs, skip_bn254_complex_mul_mem_inputs,
    skip_bn254_complex_sub_mem_inputs, skip_bn254_curve_add_mem_inputs,
    skip_bn254_curve_dbl_mem_inputs, skip_secp256k1_add_mem_inputs, skip_secp256k1_dbl_mem_inputs,
    skip_secp256r1_add_mem_inputs, skip_secp256r1_dbl_mem_inputs,
};
use crate::{ArithEqOp, ArithEqSM};

impl<F: PrimeField64> PrecompileMemInputs for ArithEqSM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        // A big-endian op is the same memory access pattern as its little-endian twin; only the
        // operand values need converting, which the generators do from `big_endian`.
        let op = ArithEqOp::from_opcode(data[OP] as u8).unwrap_or_else(|| {
            panic!("ArithEqSM::generate: unsupported sub-op {}", data[OP] as u8)
        });
        let big_endian = op.is_big_endian();
        match op.little_endian() {
            ArithEqOp::Arith256 => generate_arith256_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ArithEqOp::Arith256Mod => generate_arith256_mod_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ArithEqOp::Secp256k1Add => generate_secp256k1_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ArithEqOp::Secp256k1Dbl => generate_secp256k1_dbl_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ArithEqOp::Bn254CurveAdd => generate_bn254_curve_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ArithEqOp::Bn254CurveDbl => generate_bn254_curve_dbl_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ArithEqOp::Bn254ComplexAdd => generate_bn254_complex_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ArithEqOp::Bn254ComplexSub => generate_bn254_complex_sub_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ArithEqOp::Bn254ComplexMul => generate_bn254_complex_mul_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ArithEqOp::Secp256r1Add => generate_secp256r1_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            ArithEqOp::Secp256r1Dbl => generate_secp256r1_dbl_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                big_endian,
                mem_processors,
            ),
            _ => unreachable!("little_endian() only yields little-endian ops"),
        }
    }

    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processors: &mut P) -> bool {
        let op = ArithEqOp::from_opcode(data[OP] as u8).unwrap_or_else(|| {
            panic!("ArithEqSM::should_skip: unsupported sub-op {}", data[OP] as u8)
        });
        match op.little_endian() {
            ArithEqOp::Arith256 => skip_arith256_mem_inputs(addr_main, data, mem_processors),
            ArithEqOp::Arith256Mod => skip_arith256_mod_mem_inputs(addr_main, data, mem_processors),
            ArithEqOp::Secp256k1Add => {
                skip_secp256k1_add_mem_inputs(addr_main, data, mem_processors)
            }
            ArithEqOp::Secp256k1Dbl => {
                skip_secp256k1_dbl_mem_inputs(addr_main, data, mem_processors)
            }
            ArithEqOp::Bn254CurveAdd => {
                skip_bn254_curve_add_mem_inputs(addr_main, data, mem_processors)
            }
            ArithEqOp::Bn254CurveDbl => {
                skip_bn254_curve_dbl_mem_inputs(addr_main, data, mem_processors)
            }
            ArithEqOp::Bn254ComplexAdd => {
                skip_bn254_complex_add_mem_inputs(addr_main, data, mem_processors)
            }
            ArithEqOp::Bn254ComplexSub => {
                skip_bn254_complex_sub_mem_inputs(addr_main, data, mem_processors)
            }
            ArithEqOp::Bn254ComplexMul => {
                skip_bn254_complex_mul_mem_inputs(addr_main, data, mem_processors)
            }
            ArithEqOp::Secp256r1Add => {
                skip_secp256r1_add_mem_inputs(addr_main, data, mem_processors)
            }
            ArithEqOp::Secp256r1Dbl => {
                skip_secp256r1_dbl_mem_inputs(addr_main, data, mem_processors)
            }
            _ => unreachable!("little_endian() only yields little-endian ops"),
        }
    }
}
