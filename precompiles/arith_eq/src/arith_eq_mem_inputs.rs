use fields::PrimeField64;
use precompiles_common::{MemProcessor, PrecompileMemInputs};
use zisk_common::OP;
use zisk_core::zisk_ops::ZiskOp;

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

const ARITH256_OP: u8 = ZiskOp::Arith256.code();
const ARITH256_MOD_OP: u8 = ZiskOp::Arith256Mod.code();
const SECP256K1_ADD_OP: u8 = ZiskOp::Secp256k1Add.code();
const SECP256K1_DBL_OP: u8 = ZiskOp::Secp256k1Dbl.code();
const BN254_CURVE_ADD_OP: u8 = ZiskOp::Bn254CurveAdd.code();
const BN254_CURVE_DBL_OP: u8 = ZiskOp::Bn254CurveDbl.code();
const BN254_COMPLEX_ADD_OP: u8 = ZiskOp::Bn254ComplexAdd.code();
const BN254_COMPLEX_SUB_OP: u8 = ZiskOp::Bn254ComplexSub.code();
const BN254_COMPLEX_MUL_OP: u8 = ZiskOp::Bn254ComplexMul.code();
const SECP256R1_ADD_OP: u8 = ZiskOp::Secp256r1Add.code();
const SECP256R1_DBL_OP: u8 = ZiskOp::Secp256r1Dbl.code();

impl<F: PrimeField64> PrecompileMemInputs for ArithEqSM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        match data[OP] as u8 {
            ARITH256_OP => generate_arith256_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            ARITH256_MOD_OP => generate_arith256_mod_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            SECP256K1_ADD_OP => generate_secp256k1_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            SECP256K1_DBL_OP => generate_secp256k1_dbl_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            BN254_CURVE_ADD_OP => generate_bn254_curve_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            BN254_CURVE_DBL_OP => generate_bn254_curve_dbl_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            BN254_COMPLEX_ADD_OP => generate_bn254_complex_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            BN254_COMPLEX_SUB_OP => generate_bn254_complex_sub_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            BN254_COMPLEX_MUL_OP => generate_bn254_complex_mul_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            SECP256R1_ADD_OP => generate_secp256r1_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            SECP256R1_DBL_OP => generate_secp256r1_dbl_mem_inputs(
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
            ARITH256_OP => skip_arith256_mem_inputs(addr_main, data, mem_processors),
            ARITH256_MOD_OP => skip_arith256_mod_mem_inputs(addr_main, data, mem_processors),
            SECP256K1_ADD_OP => skip_secp256k1_add_mem_inputs(addr_main, data, mem_processors),
            SECP256K1_DBL_OP => skip_secp256k1_dbl_mem_inputs(addr_main, data, mem_processors),
            BN254_CURVE_ADD_OP => skip_bn254_curve_add_mem_inputs(addr_main, data, mem_processors),
            BN254_CURVE_DBL_OP => skip_bn254_curve_dbl_mem_inputs(addr_main, data, mem_processors),
            BN254_COMPLEX_ADD_OP => {
                skip_bn254_complex_add_mem_inputs(addr_main, data, mem_processors)
            }
            BN254_COMPLEX_SUB_OP => {
                skip_bn254_complex_sub_mem_inputs(addr_main, data, mem_processors)
            }
            BN254_COMPLEX_MUL_OP => {
                skip_bn254_complex_mul_mem_inputs(addr_main, data, mem_processors)
            }
            SECP256R1_ADD_OP => skip_secp256r1_add_mem_inputs(addr_main, data, mem_processors),
            SECP256R1_DBL_OP => skip_secp256r1_dbl_mem_inputs(addr_main, data, mem_processors),
            _ => panic!("ArithEqSM::should_skip: unsupported sub-op {}", data[OP] as u8),
        }
    }
}
