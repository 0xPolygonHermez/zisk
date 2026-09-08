use proofman_fields::PrimeField64;
use zisk_common::OP;
use zisk_core::zisk_ops::ZiskOp;
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
use crate::ArithEqSM;

impl<F: PrimeField64> PrecompileMemInputs for ArithEqSM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        match data[OP] as u8 {
            ZiskOp::ARITH256 => generate_arith256_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            ZiskOp::ARITH256_MOD => generate_arith256_mod_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            ZiskOp::SECP256K1_ADD => generate_secp256k1_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            ZiskOp::SECP256K1_DBL => generate_secp256k1_dbl_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            ZiskOp::BN254_CURVE_ADD => generate_bn254_curve_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            ZiskOp::BN254_CURVE_DBL => generate_bn254_curve_dbl_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            ZiskOp::BN254_COMPLEX_ADD => generate_bn254_complex_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            ZiskOp::BN254_COMPLEX_SUB => generate_bn254_complex_sub_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            ZiskOp::BN254_COMPLEX_MUL => generate_bn254_complex_mul_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            ZiskOp::SECP256R1_ADD => generate_secp256r1_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            ZiskOp::SECP256R1_DBL => generate_secp256r1_dbl_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            _ => panic!("ArithEqSM::generate: unsupported sub-op {}", data[OP] as u8),
        }
    }

    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processors: &mut P) -> bool {
        match data[OP] as u8 {
            ZiskOp::ARITH256 => skip_arith256_mem_inputs(addr_main, data, mem_processors),
            ZiskOp::ARITH256_MOD => skip_arith256_mod_mem_inputs(addr_main, data, mem_processors),
            ZiskOp::SECP256K1_ADD => skip_secp256k1_add_mem_inputs(addr_main, data, mem_processors),
            ZiskOp::SECP256K1_DBL => skip_secp256k1_dbl_mem_inputs(addr_main, data, mem_processors),
            ZiskOp::BN254_CURVE_ADD => {
                skip_bn254_curve_add_mem_inputs(addr_main, data, mem_processors)
            }
            ZiskOp::BN254_CURVE_DBL => {
                skip_bn254_curve_dbl_mem_inputs(addr_main, data, mem_processors)
            }
            ZiskOp::BN254_COMPLEX_ADD => {
                skip_bn254_complex_add_mem_inputs(addr_main, data, mem_processors)
            }
            ZiskOp::BN254_COMPLEX_SUB => {
                skip_bn254_complex_sub_mem_inputs(addr_main, data, mem_processors)
            }
            ZiskOp::BN254_COMPLEX_MUL => {
                skip_bn254_complex_mul_mem_inputs(addr_main, data, mem_processors)
            }
            ZiskOp::SECP256R1_ADD => skip_secp256r1_add_mem_inputs(addr_main, data, mem_processors),
            ZiskOp::SECP256R1_DBL => skip_secp256r1_dbl_mem_inputs(addr_main, data, mem_processors),
            _ => panic!("ArithEqSM::should_skip: unsupported sub-op {}", data[OP] as u8),
        }
    }
}
