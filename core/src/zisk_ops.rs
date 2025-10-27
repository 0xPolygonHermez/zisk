//! * Defines the operations that can be executed in Zisk as part of an instruction.
//! * The macro `define_ops` is used to define every operation, including its opcode, human-readable
//!   name, type, etc.
//! * The opcode operation functions are called `op_<opcode>`, they accept 2 input parameters a and
//!   b, and return 2 output results c and flag.
//! * The `opc_<opcode>` functions are wrappers over the `op_<opcode>` functions that accept an
//!   `InstContext` (instruction context) as input/output parameter, containg a, b, c and flag
//!   attributes.

#![allow(unused)]

use ziskos::fcall_proxy;

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    num::Wrapping,
    str::FromStr,
};
use tiny_keccak::keccakf;

use crate::{
    sha256f, EmulationMode, InstContext, Mem, ZiskOperationType, ZiskRequiredOperation, M64,
    REG_A0, SYS_ADDR,
};

use lib_c::{inverse_fn_ec_c, inverse_fp_ec_c, sqrt_fp_ec_parity_c, Fcall, FcallContext};

use crate::{
    FCALL_ID_INVERSE_FN_EC, FCALL_ID_INVERSE_FP_EC, FCALL_ID_SQRT_FP_EC_PARITY,
    FCALL_PARAMS_MAX_SIZE, FCALL_RESULT_MAX_SIZE,
};

/// Determines the type of a [`ZiskOp`].
///
/// The type will be used to assign the proof generation of a main state machine operation result to
/// the corresponding secondary state machine.
/// The type can be: internal (no proof required), arith, binary, etc.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OpType {
    Internal,
    Arith,
    ArithA32,
    ArithAm32,
    Binary,
    BinaryE,
    Keccak,
    Sha256,
    PubOut,
    ArithEq,
    Fcall,
    ArithEq384,
    BigInt,
}

impl From<OpType> for ZiskOperationType {
    fn from(op_type: OpType) -> Self {
        match op_type {
            OpType::Internal => ZiskOperationType::Internal,
            OpType::Arith | OpType::ArithA32 | OpType::ArithAm32 => ZiskOperationType::Arith,
            OpType::Binary => ZiskOperationType::Binary,
            OpType::BinaryE => ZiskOperationType::BinaryE,
            OpType::Keccak => ZiskOperationType::Keccak,
            OpType::Sha256 => ZiskOperationType::Sha256,
            OpType::PubOut => ZiskOperationType::PubOut,
            OpType::ArithEq => ZiskOperationType::ArithEq,
            OpType::Fcall => ZiskOperationType::Fcall,
            OpType::ArithEq384 => ZiskOperationType::ArithEq384,
            OpType::BigInt => ZiskOperationType::BigInt,
        }
    }
}

impl Display for OpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal => write!(f, "i"),
            Self::Arith => write!(f, "a"),
            Self::ArithA32 => write!(f, "a32"),
            Self::ArithAm32 => write!(f, "am32"),
            Self::Binary => write!(f, "b"),
            Self::BinaryE => write!(f, "BinaryE"),
            Self::Keccak => write!(f, "Keccak"),
            Self::Sha256 => write!(f, "Sha256"),
            Self::PubOut => write!(f, "PubOut"),
            Self::ArithEq => write!(f, "Arith256"),
            Self::Fcall => write!(f, "Fcall"),
            Self::ArithEq384 => write!(f, "Arith384"),
            Self::BigInt => write!(f, "BigInt"),
        }
    }
}

impl FromStr for OpType {
    type Err = InvalidOpTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "i" => Ok(Self::Internal),
            "a" => Ok(Self::Arith),
            "a32" => Ok(Self::ArithA32),
            "am32" => Ok(Self::ArithAm32),
            "b" => Ok(Self::Binary),
            "be" => Ok(Self::BinaryE),
            "k" => Ok(Self::Keccak),
            "s" => Ok(Self::Sha256),
            "aeq" => Ok(Self::ArithEq),
            "fcall" => Ok(Self::Fcall),
            "aeq384" => Ok(Self::ArithEq384),
            "bint" => Ok(Self::BigInt),
            _ => Err(InvalidOpTypeError),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InvalidOpTypeError;

impl Display for InvalidOpTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid operation type")
    }
}

#[derive(Copy, Clone, Debug)]
pub struct InvalidNameError;

impl Display for InvalidNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid op name")
    }
}

#[derive(Copy, Clone, Debug)]
pub struct InvalidCodeError;

impl Display for InvalidCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid op code")
    }
}
/// Trait for gathering operation statistics
pub trait OpStats {
    fn mem_align_read(&mut self, addr: u64, count: usize);
    fn mem_align_write(&mut self, addr: u64, count: usize);
}

/// Stats gathering function that does nothing (used as default)
#[inline(always)]
pub fn ops_none(_ctx: &InstContext, _stats: &mut dyn OpStats) {
    // No-op implementation
}

/// Internal macro used to define all ops in the [`ZiskOp`] enum
macro_rules! define_ops {
    ( $( ($name:ident, $str_name:expr, $type:ident, $steps:expr, $code:expr, $input_size:expr, $output_size:expr, $call_fn:ident, $call_ab_fn:ident, $call_stats_fn:ident ) ),* $(,)? ) => {
		/// Represents an operation that can be executed in Zisk.
		///
		/// All relevant metadata associated with the operation can be efficiently accessed via
		/// the const methods on this enum.
        #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum ZiskOp {
            $(
                $name = $code,
            )*
        }

        impl ZiskOp {
			/// Returns the (string) name of the operation
            pub const fn name(&self) -> &'static str {
                match self {
                    $(
                        Self::$name => $str_name,
                    )*
                }
            }

			/// Returns the [`OpType`] of the operation
            pub const fn op_type(&self) -> OpType {
                match self {
                    $(
                        Self::$name => OpType::$type,
                    )*
                }
            }

			/// Returns the number of steps required to execute the operation
            pub const fn steps(&self) -> u64 {
                match self {
                    $(
                        Self::$name => $steps,
                    )*
                }
            }

			/// Returns the raw op code of the operation
            pub const fn code(&self) -> u8 {
                match self {
                    $(
                        Self::$name => $code,
                    )*
                }
            }

			/// Returns the input data size of the operation
            pub const fn input_size(&self) -> u64 {
                match self {
                    $(
                        Self::$name => $input_size,
                    )*
                }
            }

			/// Returns the input data size of the operation
            pub const fn output_size(&self) -> u64 {
                match self {
                    $(
                        Self::$name => $output_size,
                    )*
                }
            }

            /// Executes the operation on the given [`InstContext`]
			#[inline(always)]
            pub fn call(&self, ctx: &mut InstContext) {
                match self {
                    $(
                        Self::$name => $call_fn(ctx),
                    )*
                }
            }

            /// Returns the call function of the operation
            pub const fn get_call_function(&self) -> fn(&mut InstContext) -> () {
                match self {
                    $(
                        Self::$name => $call_fn,
                    )*
                }
            }

            /// Executes the operation to obtain extra stats on the given [`InstContext`]
            #[inline(always)]
            pub fn call_stats(&self, ctx: &InstContext, stats: &mut dyn OpStats) {
                match self {
                    $(
                        Self::$name => $call_stats_fn(ctx, stats),
                    )*
                }
            }

            /// Returns the call function for extra stats
            pub const fn get_call_stats_function(&self) -> fn(&InstContext, &mut dyn OpStats) -> () {
                match self {
                    $(
                        Self::$name => $call_stats_fn,
                    )*
                }
            }


			/// Executes the operation on the given inputs `a` and `b`
			#[inline(always)]
            pub fn call_ab(&self, a: u64, b: u64) -> (u64, bool) {
                match self {
                    $(
                        Self::$name => $call_ab_fn(a, b),
                    )*
                }
            }

			/// Attempts to create a [`ZiskOp`] from a string name, returning an error if the
			/// name is invalid
            pub fn try_from_name(st: &str) -> Result<ZiskOp, InvalidNameError> {
                match st {
                    $(
                        $str_name => Ok(Self::$name),
                    )*
                    _ => Err(InvalidNameError)
                }
            }

			/// Attempts to create a [`ZiskOp`] from a raw op code, returning an error if the
			/// code is invalid
            pub const fn try_from_code(code: u8) -> Result<ZiskOp, InvalidCodeError> {
                match code {
                    $(
                        $code => Ok(Self::$name),
                    )*
                    _ => Err(InvalidCodeError)
                }
            }

			/// Executes opcodes, only if it does not require instruction context (e.g. it does
			/// not have to access memory).
			///
			/// Panics if the opcode is invalid or does not support this operation.
			#[inline(always)]
			pub fn execute(code: u8, a: u64, b: u64) -> (u64, bool) {
				match code {
					$(
						$code => Self::$name.call_ab(a, b),
					)*
					_ => panic!("Invalid opcode: {}", code),
				}
			}
        }
    };
}

// Cost definitions: Area x Op
const INTERNAL_COST: u64 = 0;
const BINARY_COST: u64 = 75;
const BINARY_ADD_COST: u64 = 26;
const BINARY_E_COST: u64 = 54;
const ARITHA32_COST: u64 = 95;
const ARITHAM32_COST: u64 = 95;
const KECCAK_COST: u64 = (((93846 * 86) - 1) / 63) + 1;
const SHA256_COST: u64 = 72 * 121;
const ARITH_EQ_COST: u64 = 85 * 16;
const FCALL_COST: u64 = INTERNAL_COST;
const ARITH_EQ_384_COST: u64 = 79 * 24;
const ADD256_COST: u64 = 104;

/// Table of Zisk opcode definitions: enum, name, type, cost, code and implementation functions
/// This table is the backbone of the Zisk processor, it determines what functionality is supported,
/// and what state machine is responsible of proving the execution of every opcode, based on its
/// type.
define_ops! {
    (Flag, "flag", Internal, INTERNAL_COST, 0x00, 0, 0, opc_flag, op_flag, ops_none),
    (CopyB, "copyb", Internal, INTERNAL_COST, 0x01, 0, 0, opc_copyb, op_copyb, ops_none),
    (SignExtendB, "signextend_b", BinaryE, BINARY_E_COST, 0x37, 0, 0, opc_signextend_b, op_signextend_b, ops_none),
    (SignExtendH, "signextend_h", BinaryE, BINARY_E_COST, 0x38, 0, 0, opc_signextend_h, op_signextend_h, ops_none),
    (SignExtendW, "signextend_w", BinaryE, BINARY_E_COST, 0x39, 0, 0, opc_signextend_w, op_signextend_w, ops_none),
    (Add, "add", Binary, BINARY_ADD_COST, 0x0c, 0, 0, opc_add, op_add, ops_none),
    (AddW, "add_w", Binary, BINARY_COST, 0x2c, 0, 0, opc_add_w, op_add_w, ops_none),
    (Sub, "sub", Binary, BINARY_COST, 0x0d, 0, 0, opc_sub, op_sub, ops_none),
    (SubW, "sub_w", Binary, BINARY_COST, 0x2d, 0, 0, opc_sub_w, op_sub_w, ops_none),
    (Sll, "sll", BinaryE, BINARY_E_COST, 0x31, 0, 0, opc_sll, op_sll, ops_none),
    (SllW, "sll_w", BinaryE, BINARY_E_COST, 0x34, 0, 0, opc_sll_w, op_sll_w, ops_none),
    (Sra, "sra", BinaryE, BINARY_E_COST, 0x33, 0, 0, opc_sra, op_sra, ops_none),
    (Srl, "srl", BinaryE, BINARY_E_COST, 0x32, 0, 0, opc_srl, op_srl, ops_none),
    (SraW, "sra_w", BinaryE, BINARY_E_COST, 0x36, 0, 0, opc_sra_w, op_sra_w, ops_none),
    (SrlW, "srl_w", BinaryE, BINARY_E_COST, 0x35, 0, 0, opc_srl_w, op_srl_w, ops_none),
    (Eq, "eq", Binary, BINARY_COST, 0x0b, 0, 0, opc_eq, op_eq, ops_none),
    (EqW, "eq_w", Binary, BINARY_COST, 0x2b, 0, 0, opc_eq_w, op_eq_w, ops_none),
    (Ltu, "ltu", Binary, BINARY_COST, 0x08, 0, 0, opc_ltu, op_ltu, ops_none),
    (Lt, "lt", Binary, BINARY_COST, 0x09, 0, 0, opc_lt, op_lt, ops_none),
    (LtuW, "ltu_w", Binary, BINARY_COST, 0x28, 0, 0, opc_ltu_w, op_ltu_w, ops_none),
    (LtW, "lt_w", Binary, BINARY_COST, 0x29, 0, 0, opc_lt_w, op_lt_w, ops_none),
    (Leu, "leu", Binary, BINARY_COST, 0x0e, 0, 0, opc_leu, op_leu, ops_none),
    (Le, "le", Binary, BINARY_COST, 0x0f, 0, 0, opc_le, op_le, ops_none),
    (LeuW, "leu_w", Binary, BINARY_COST, 0x2e, 0, 0, opc_leu_w, op_leu_w, ops_none),
    (LeW, "le_w", Binary, BINARY_COST, 0x2f, 0, 0, opc_le_w, op_le_w, ops_none),
    (And, "and", Binary, BINARY_COST, 0x10, 0, 0, opc_and, op_and, ops_none),
    (Or, "or", Binary, BINARY_COST, 0x11, 0, 0, opc_or, op_or, ops_none),
    (Xor, "xor", Binary, BINARY_COST, 0x12, 0, 0, opc_xor, op_xor, ops_none),
    (Mulu, "mulu", ArithAm32, ARITHAM32_COST, 0xb0, 0, 0, opc_mulu, op_mulu, ops_none),
    (Muluh, "muluh", ArithAm32, ARITHAM32_COST, 0xb1, 0, 0, opc_muluh, op_muluh, ops_none),
    (Mulsuh, "mulsuh", ArithAm32, ARITHAM32_COST, 0xb3, 0, 0, opc_mulsuh, op_mulsuh, ops_none),
    (Mul, "mul", ArithAm32, ARITHAM32_COST, 0xb4, 0, 0, opc_mul, op_mul, ops_none),
    (Mulh, "mulh", ArithAm32, ARITHAM32_COST, 0xb5, 0, 0, opc_mulh, op_mulh, ops_none),
    (MulW, "mul_w", ArithAm32, ARITHAM32_COST, 0xb6, 0, 0, opc_mul_w, op_mul_w, ops_none),
    (Divu, "divu", ArithAm32, ARITHAM32_COST, 0xb8, 0, 0, opc_divu, op_divu, ops_none),
    (Remu, "remu", ArithAm32, ARITHAM32_COST, 0xb9, 0, 0, opc_remu, op_remu, ops_none),
    (Div, "div", ArithAm32, ARITHAM32_COST, 0xba, 0, 0, opc_div, op_div, ops_none),
    (Rem, "rem", ArithAm32, ARITHAM32_COST, 0xbb, 0, 0, opc_rem, op_rem, ops_none),
    (DivuW, "divu_w", ArithA32, ARITHA32_COST, 0xbc, 0, 0, opc_divu_w, op_divu_w, ops_none),
    (RemuW, "remu_w", ArithA32, ARITHA32_COST, 0xbd, 0, 0, opc_remu_w, op_remu_w, ops_none),
    (DivW, "div_w", ArithA32, ARITHA32_COST, 0xbe, 0, 0, opc_div_w, op_div_w, ops_none),
    (RemW, "rem_w", ArithA32, ARITHA32_COST, 0xbf, 0, 0, opc_rem_w, op_rem_w, ops_none),
    (Minu, "minu", Binary, BINARY_COST, 0x02, 0, 0, opc_minu, op_minu, ops_none),
    (Min, "min", Binary, BINARY_COST, 0x03, 0, 0, opc_min, op_min, ops_none),
    (MinuW, "minu_w", Binary, BINARY_COST, 0x22, 0, 0, opc_minu_w, op_minu_w, ops_none),
    (MinW, "min_w", Binary, BINARY_COST, 0x23, 0, 0, opc_min_w, op_min_w, ops_none),
    (Maxu, "maxu", Binary, BINARY_COST, 0x04, 0, 0, opc_maxu, op_maxu, ops_none),
    (Max, "max", Binary, BINARY_COST, 0x05, 0, 0, opc_max, op_max, ops_none),
    (MaxuW, "maxu_w", Binary, BINARY_COST, 0x24, 0, 0, opc_maxu_w, op_maxu_w, ops_none),
    (MaxW, "max_w", Binary, BINARY_COST, 0x25, 0, 0, opc_max_w, op_max_w, ops_none),
    (Keccak, "keccak", Keccak, KECCAK_COST, 0xf1, 200, 200, opc_keccak, op_keccak, ops_none),
    (PubOut, "pubout", PubOut, 0, 0x30, 0, 0, opc_pubout, op_pubout, ops_none),
    (Add256, "add256", BigInt, ADD256_COST, 0xf0, 104, 32, opc_add256, op_add256, ops_add256),
    (Arith256, "arith256", ArithEq, ARITH_EQ_COST, 0xf2, 136, 64, opc_arith256, op_arith256, ops_arith256),
    (Arith256Mod, "arith256_mod", ArithEq, ARITH_EQ_COST, 0xf3, 168, 32, opc_arith256_mod, op_arith256_mod, ops_arith256_mod),
    (Secp256k1Add, "secp256k1_add", ArithEq, ARITH_EQ_COST, 0xf4, 144, 64, opc_secp256k1_add, op_secp256k1_add, ops_secp256k1_add),
    (Secp256k1Dbl, "secp256k1_dbl", ArithEq, ARITH_EQ_COST, 0xf5, 64, 64, opc_secp256k1_dbl, op_secp256k1_add, ops_secp256k1_dbl),
    (FcallParam, "fcall_param", Fcall, FCALL_COST, 0xf6, 0, 0, opc_fcall_param, op_fcall_param, ops_none),
    (Fcall, "fcall", Fcall, FCALL_COST, 0xf7, 0, 0, opc_fcall, op_fcall, ops_none),
    (FcallGet, "fcall_get", Fcall, FCALL_COST, 0xf8, 0, 0, opc_fcall_get, op_fcall_get, ops_none),
    (Sha256, "sha256", Sha256, SHA256_COST, 0xf9, 112, 112, opc_sha256, op_sha256, ops_sha256),
    (Bn254CurveAdd, "bn254_curve_add", ArithEq, ARITH_EQ_COST, 0xfa, 144, 64, opc_bn254_curve_add, op_bn254_curve_add, ops_bn254_curve_add),
    (Bn254CurveDbl, "bn254_curve_dbl", ArithEq, ARITH_EQ_COST, 0xfb, 64, 64, opc_bn254_curve_dbl, op_bn254_curve_dbl, ops_bn254_curve_dbl),
    (Bn254ComplexAdd, "bn254_complex_add", ArithEq, ARITH_EQ_COST, 0xfc, 144, 64, opc_bn254_complex_add, op_bn254_complex_add, ops_bn254_complex_add),
    (Bn254ComplexSub, "bn254_complex_sub", ArithEq, ARITH_EQ_COST, 0xfd, 144, 64, opc_bn254_complex_sub, op_bn254_complex_sub, ops_bn254_complex_sub),
    (Bn254ComplexMul, "bn254_complex_mul", ArithEq, ARITH_EQ_COST, 0xfe, 144, 64, opc_bn254_complex_mul, op_bn254_complex_mul, ops_bn254_complex_mul),
    (Halt, "halt", Internal, INTERNAL_COST, 0xff, 144, 0, opc_halt, op_halt, ops_none),
    (Arith384Mod, "arith384_mod", ArithEq384, ARITH_EQ_384_COST, 0xe2, 232, 48, opc_arith384_mod, op_arith384_mod, ops_arith384_mod),
    (Bls12_381CurveAdd, "bls12_381_curve_add", ArithEq384, ARITH_EQ_384_COST, 0xe3, 208, 96, opc_bls12_381_curve_add, op_bls12_381_curve_add, ops_bls12_381_curve_add),
    (Bls12_381CurveDbl, "bls12_381_curve_dbl", ArithEq384, ARITH_EQ_384_COST, 0xe4, 96, 96, opc_bls12_381_curve_dbl, op_bls12_381_curve_dbl, ops_bls12_381_curve_dbl),
    (Bls12_381ComplexAdd, "bls12_381_complex_add", ArithEq384, ARITH_EQ_384_COST, 0xe5, 208, 96, opc_bls12_381_complex_add, op_bls12_381_complex_add, ops_bls12_381_complex_add),
    (Bls12_381ComplexSub, "bls12_381_complex_sub", ArithEq384, ARITH_EQ_384_COST, 0xe6, 208, 96, opc_bls12_381_complex_sub, op_bls12_381_complex_sub, ops_bls12_381_complex_sub),
    (Bls12_381ComplexMul, "bls12_381_complex_mul", ArithEq384, ARITH_EQ_384_COST, 0xe7, 208, 96, opc_bls12_381_complex_mul, op_bls12_381_complex_mul, ops_bls12_381_complex_mul),
}

/* INTERNAL operations */

/// Sets flag to true (and c to 0)
#[inline(always)]
pub const fn op_flag(_a: u64, _b: u64) -> (u64, bool) {
    (0, true)
}

/// InstContext-based wrapper over op_flag()
#[inline(always)]
pub fn opc_flag(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_flag(ctx.a, ctx.b);
}

/// Copies register b into c (and flag to false)
#[inline(always)]
pub const fn op_copyb(_a: u64, b: u64) -> (u64, bool) {
    (b, false)
}

/// InstContext-based wrapper over op_copyb()
#[inline(always)]
pub fn opc_copyb(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_copyb(ctx.a, ctx.b);
}

/* SIGN EXTEND operations for different data widths (i8, i16 and i32) --> i64 --> u64 */

/// Sign extends an i8.
///
/// Converts b from a signed 8-bits number in the range [-128, +127] into a signed 64-bit number of
/// the same value, adding 0xFFFFFFFFFFFFFF00 if negative, and stores the result in c as a u64 (and
/// sets flag to false)
#[inline(always)]
pub const fn op_signextend_b(_a: u64, b: u64) -> (u64, bool) {
    ((b as i8) as u64, false)
}

/// InstContext-based wrapper over op_signextend_b()
#[inline(always)]
pub fn opc_signextend_b(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_signextend_b(ctx.a, ctx.b);
}

/// Sign extends an i16.
///
/// Converts b from a signed 16-bits number in the range [-32768, 32767] into a signed 64-bit number
/// of the same value, adding 0xFFFFFFFFFFFF0000 if negative, and stores the result in c as a u64
/// (and sets flag to false)
#[inline(always)]
pub const fn op_signextend_h(_a: u64, b: u64) -> (u64, bool) {
    ((b as i16) as u64, false)
}

/// InstContext-based wrapper over op_signextend_h()
#[inline(always)]
pub fn opc_signextend_h(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_signextend_h(ctx.a, ctx.b);
}

/// Sign extends an i32.
///
/// Converts b from a signed 32-bits number in the range [-2147483648, 2147483647] into a signed
/// 64-bit number of the same value, adding 0xFFFFFFFF00000000 if negative  and stores the result in
/// c as a u64 (and sets flag to false)
#[inline(always)]
pub const fn op_signextend_w(_a: u64, b: u64) -> (u64, bool) {
    ((b as i32) as u64, false)
}

/// InstContext-based wrapper over op_signextend_w()
#[inline(always)]
pub fn opc_signextend_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_signextend_w(ctx.a, ctx.b);
}

/* ADD AND SUB operations for different data widths (i32 and u64) */

/// Adds a and b as 64-bit unsigned values, and stores the result in c (and sets flag to false)
#[inline(always)]
pub fn op_add(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) + Wrapping(b)).0, false)
}

/// InstContext-based wrapper over op_add()
#[inline(always)]
pub fn opc_add(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_add(ctx.a, ctx.b);
}

/// Adds a and b as 32-bit signed values, and stores the result in c (and flag to false)
#[inline(always)]
pub fn op_add_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) + Wrapping(b as i32)).0 as u64, false)
}

/// InstContext-based wrapper over op_add_w()
#[inline(always)]
pub fn opc_add_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_add_w(ctx.a, ctx.b);
}

/// Subtracts a and b as 64-bit unsigned values, and stores the result in c (and sets flag to false)
#[inline(always)]
pub fn op_sub(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) - Wrapping(b)).0, false)
}

/// InstContext-based wrapper over op_sub()
#[inline(always)]
pub fn opc_sub(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sub(ctx.a, ctx.b);
}

/// Subtracts a and b as 32-bit signed values, and stores the result in c (and sets flag to false)
#[inline(always)]
pub fn op_sub_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) - Wrapping(b as i32)).0 as u64, false)
}

/// InstContext-based wrapper over op_sub_w()
#[inline(always)]
pub fn opc_sub_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sub_w(ctx.a, ctx.b);
}

/* SHIFT operations */

/// Shifts a as a 64-bits unsigned value to the left b mod 64 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub const fn op_sll(a: u64, b: u64) -> (u64, bool) {
    (a << (b & 0x3f), false)
}

/// InstContext-based wrapper over op_sll()
#[inline(always)]
pub fn opc_sll(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sll(ctx.a, ctx.b);
}

/// Shifts a as a 32-bits unsigned value to the left b mod 64 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub fn op_sll_w(a: u64, b: u64) -> (u64, bool) {
    (((Wrapping(a as u32) << (b & 0x3f) as usize).0 as i32) as u64, false)
}

/// InstContext-based wrapper over op_sll_w()
#[inline(always)]
pub fn opc_sll_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sll_w(ctx.a, ctx.b);
}

/// Shifts a as a 64-bits signed value to the right b mod 64 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub const fn op_sra(a: u64, b: u64) -> (u64, bool) {
    (((a as i64) >> (b & 0x3f)) as u64, false)
}

/// InstContext-based wrapper over op_sra()
#[inline(always)]
pub fn opc_sra(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sra(ctx.a, ctx.b);
}

/// Shifts a as a 64-bits unsigned value to the right b mod 64 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub const fn op_srl(a: u64, b: u64) -> (u64, bool) {
    (a >> (b & 0x3f), false)
}

/// InstContext-based wrapper over op_srl()
#[inline(always)]
pub fn opc_srl(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_srl(ctx.a, ctx.b);
}

/// Shifts a as a 32-bits signed value to the right b mod 64 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub fn op_sra_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) >> (b & 0x3f) as usize).0 as u64, false)
}

/// InstContext-based wrapper over op_sra_w()
#[inline(always)]
pub fn opc_sra_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_sra_w(ctx.a, ctx.b);
}

/// Shifts a as a 32-bits unsigned value to the right b mod 64 bits, and stores the result in c (and
/// sets flag to false)
#[inline(always)]
pub fn op_srl_w(a: u64, b: u64) -> (u64, bool) {
    (((Wrapping(a as u32) >> (b & 0x3f) as usize).0 as i32) as u64, false)
}

/// InstContext-based wrapper over op_srl_w()
#[inline(always)]
pub fn opc_srl_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_srl_w(ctx.a, ctx.b);
}

/* COMPARISON operations */

/// If a and b are equal, it returns c=1, flag=true; otherwise it returns c=0, flag=false
#[inline(always)]
pub const fn op_eq(a: u64, b: u64) -> (u64, bool) {
    if a == b {
        (1, true)
    } else {
        (0, false)
    }
}

/// InstContext-based wrapper over op_eq()
#[inline(always)]
pub fn opc_eq(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_eq(ctx.a, ctx.b);
}

/// If a and b as 32-bit signed values are equal, as 64-bit unsigned values, it returns c=1,
/// flag=true; otherwise it returns c=0, flag=false
#[inline(always)]
pub const fn op_eq_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) == (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}

/// InstContext-based wrapper over op_eq_w()
#[inline(always)]
pub fn opc_eq_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_eq_w(ctx.a, ctx.b);
}

/// If a is strictly less than b, as 64-bit unsigned values, it returns c=1, flag=true; otherwise it
/// returns c=0, flag=false
#[inline(always)]
pub const fn op_ltu(a: u64, b: u64) -> (u64, bool) {
    if a < b {
        (1, true)
    } else {
        (0, false)
    }
}

/// InstContext-based wrapper over op_ltu()
#[inline(always)]
pub fn opc_ltu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_ltu(ctx.a, ctx.b);
}

/// If a is strictly less than b, as 64-bit signed values, it returns c=1, flag=true; otherwise it
/// returns c=0, flag=false
#[inline(always)]
pub const fn op_lt(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) < (b as i64) {
        (1, true)
    } else {
        (0, false)
    }
}

/// InstContext-based wrapper over op_lt()
#[inline(always)]
pub fn opc_lt(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_lt(ctx.a, ctx.b);
}

/// If a is strictly less than b, as 32-bit unsigned values, it returns c=1, flag=true; otherwise it
/// returns c=0, flag=false
#[inline(always)]
pub const fn op_ltu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) < (b as u32) {
        (1, true)
    } else {
        (0, false)
    }
}

/// InstContext-based wrapper over op_ltu_w()
#[inline(always)]
pub fn opc_ltu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_ltu_w(ctx.a, ctx.b);
}

/// If a is strictly less than b, as 32-bit signed values, it returns c=1, flag=true; otherwise it
/// returns c=0, flag=false
#[inline(always)]
pub const fn op_lt_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) < (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}

/// InstContext-based wrapper over op_lt_w()
#[inline(always)]
pub fn opc_lt_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_lt_w(ctx.a, ctx.b);
}

/// If a is less than or equal to b, as 64-bit unsigned values, it returns c=1, flag=true; otherwise
/// it returns c=0, flag=false
#[inline(always)]
pub const fn op_leu(a: u64, b: u64) -> (u64, bool) {
    if a <= b {
        (1, true)
    } else {
        (0, false)
    }
}

/// InstContext-based wrapper over op_leu()
#[inline(always)]
pub fn opc_leu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_leu(ctx.a, ctx.b);
}

/// If a is less than or equal to b, as 64-bit signed values, it returns c=1, flag=true; otherwise
/// it returns c=0, flag=false
#[inline(always)]
pub const fn op_le(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) <= (b as i64) {
        (1, true)
    } else {
        (0, false)
    }
}

/// InstContext-based wrapper over op_le()
#[inline(always)]
pub fn opc_le(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_le(ctx.a, ctx.b);
}

/// If a is less than or equal to b, as 32-bit unsigned values, it returns c=1, flag=true; otherwise
/// it returns c=0, flag=false
#[inline(always)]
pub const fn op_leu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) <= (b as u32) {
        (1, true)
    } else {
        (0, false)
    }
}

/// InstContext-based wrapper over op_leu_w()
#[inline(always)]
pub fn opc_leu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_leu_w(ctx.a, ctx.b);
}

/// If a is less than or equal to b, as 32-bit signed values, it returns c=1, flag=true; otherwise
/// it returns c=0, flag=false
#[inline(always)]
pub const fn op_le_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) <= (b as i32) {
        (1, true)
    } else {
        (0, false)
    }
}

/// InstContext-based wrapper over op_le_w()
#[inline(always)]
pub fn opc_le_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_le_w(ctx.a, ctx.b);
}

/* LOGICAL operations */

/// Sets c to a AND b, and flag to false
#[inline(always)]
pub const fn op_and(a: u64, b: u64) -> (u64, bool) {
    (a & b, false)
}

/// InstContext-based wrapper over op_and()
#[inline(always)]
pub fn opc_and(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_and(ctx.a, ctx.b);
}

/// Sets c to a OR b, and flag to false
#[inline(always)]
pub const fn op_or(a: u64, b: u64) -> (u64, bool) {
    (a | b, false)
}

/// InstContext-based wrapper over op_or()
#[inline(always)]
pub fn opc_or(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_or(ctx.a, ctx.b);
}

/// Sets c to a XOR b, and flag to false
#[inline(always)]
pub const fn op_xor(a: u64, b: u64) -> (u64, bool) {
    (a ^ b, false)
}

/// InstContext-based wrapper over op_xor()
#[inline(always)]
pub fn opc_xor(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_xor(ctx.a, ctx.b);
}

/* ARITHMETIC operations: div / mul / rem */

/// Sets c to a x b, as 64-bits unsigned values, and flag to false
#[inline(always)]
pub fn op_mulu(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) * Wrapping(b)).0, false)
}

/// InstContext-based wrapper over op_mulu()
#[inline(always)]
pub fn opc_mulu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mulu(ctx.a, ctx.b);
}

/// Sets c to a x b, as 64-bits signed values, and flag to false
#[inline(always)]
pub fn op_mul(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i64) * Wrapping(b as i64)).0 as u64, false)
}

/// InstContext-based wrapper over op_mul()
#[inline(always)]
pub fn opc_mul(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mul(ctx.a, ctx.b);
}

/// Sets c to a x b, as 32-bits signed values, and flag to false
#[inline(always)]
pub fn op_mul_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) * Wrapping(b as i32)).0 as u64, false)
}

/// InstContext-based wrapper over op_mul_w()
#[inline(always)]
pub fn opc_mul_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mul_w(ctx.a, ctx.b);
}

/// Sets c to the highest 64-bits of a x b, as 128-bits unsigned values, and flag to false
#[inline(always)]
pub const fn op_muluh(a: u64, b: u64) -> (u64, bool) {
    (((a as u128 * b as u128) >> 64) as u64, false)
}

/// InstContext-based wrapper over op_muluh()
#[inline(always)]
pub fn opc_muluh(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_muluh(ctx.a, ctx.b);
}

/// Sets c to the highest 64-bits of a x b, as 128-bits unsigned values, and flag to false
#[inline(always)]
pub const fn op_mulh(a: u64, b: u64) -> (u64, bool) {
    (((((a as i64) as i128) * ((b as i64) as i128)) >> 64) as u64, false)
}

/// InstContext-based wrapper over op_mulh()
#[inline(always)]
pub fn opc_mulh(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mulh(ctx.a, ctx.b);
}

/// Sets c to the highest 64-bits of a x b, as 128-bits signed values, and flag to false
#[inline(always)]
pub const fn op_mulsuh(a: u64, b: u64) -> (u64, bool) {
    (((((a as i64) as i128) * (b as i128)) >> 64) as u64, false)
}

/// InstContext-based wrapper over op_mulsuh()
#[inline(always)]
pub fn opc_mulsuh(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_mulsuh(ctx.a, ctx.b);
}

/// Sets c to a / b, as 64-bits unsigned values, and flag to false.
/// If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
#[inline(always)]
pub const fn op_divu(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (M64, true);
    }

    (a / b, false)
}

/// InstContext-based wrapper over op_divu()
#[inline(always)]
pub fn opc_divu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_divu(ctx.a, ctx.b);
}

/// Sets c to a / b, as 64-bits signed values, and flag to false.
///
/// If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
/// If a=0x8000000000000000 (MIN_I64) and b=0xFFFFFFFFFFFFFFFF (-1) the result should be -MIN_I64,
/// which cannot be represented with 64 bits (overflow) and it returns c=a.
#[inline(always)]
pub const fn op_div(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (M64, true);
    }
    ((((a as i64) as i128) / ((b as i64) as i128)) as u64, false)
}

/// InstContext-based wrapper over op_div()
#[inline(always)]
pub fn opc_div(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_div(ctx.a, ctx.b);
}

/// Sets c to a / b, as 32-bits unsigned values, and flag to false.
/// If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
#[inline(always)]
pub const fn op_divu_w(a: u64, b: u64) -> (u64, bool) {
    if b as u32 == 0 {
        return (M64, true);
    }

    (((a as u32 / b as u32) as i32) as u64, false)
}

/// InstContext-based wrapper over op_divu_w()
#[inline(always)]
pub fn opc_divu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_divu_w(ctx.a, ctx.b);
}

/// Sets c to a / b, as 32-bits signed values, and flag to false.
/// If b=0 (divide by zero) it sets c to 2^64 - 1, and sets flag to true.
#[inline(always)]
pub const fn op_div_w(a: u64, b: u64) -> (u64, bool) {
    if b as i32 == 0 {
        return (M64, true);
    }

    ((((a as i32) as i64) / ((b as i32) as i64)) as u64, false)
}

/// InstContext-based wrapper over op_div_w()
#[inline(always)]
pub fn opc_div_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_div_w(ctx.a, ctx.b);
}

/// Sets c to a mod b, as 64-bits unsigned values, and flag to false.
/// If b=0 (divide by zero) it sets c to a, and sets flag to true.
#[inline(always)]
pub const fn op_remu(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (a, true);
    }

    (a % b, false)
}

/// InstContext-based wrapper over op_remu()
#[inline(always)]
pub fn opc_remu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_remu(ctx.a, ctx.b);
}

/// Sets c to a mod b, as 64-bits signed values, and flag to false.
/// If b=0 (divide by zero) it sets c to a, and sets flag to true.
#[inline(always)]
pub const fn op_rem(a: u64, b: u64) -> (u64, bool) {
    if b == 0 {
        return (a, true);
    }

    ((((a as i64) as i128) % ((b as i64) as i128)) as u64, false)
}

/// InstContext-based wrapper over op_rem()
#[inline(always)]
pub fn opc_rem(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_rem(ctx.a, ctx.b);
}

/// Sets c to a mod b, as 32-bits unsigned values, and flag to false.
/// If b=0 (divide by zero) it sets c to a, and sets flag to true.
#[inline(always)]
pub const fn op_remu_w(a: u64, b: u64) -> (u64, bool) {
    if (b as u32) == 0 {
        return ((a as i32) as u64, true);
    }

    ((((a as u32) % (b as u32)) as i32) as u64, false)
}

/// InstContext-based wrapper over op_remu_w()
#[inline(always)]
pub fn opc_remu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_remu_w(ctx.a, ctx.b);
}

/// Sets c to a mod b, as 32-bits signed values, and flag to false.
/// If b=0 (divide by zero) it sets c to a, and sets flag to true.
#[inline(always)]
pub const fn op_rem_w(a: u64, b: u64) -> (u64, bool) {
    if (b as i32) == 0 {
        return ((a as i32) as u64, true);
    }

    ((((a as i32) as i64) % ((b as i32) as i64)) as u64, false)
}

/// InstContext-based wrapper over op_rem_w()
#[inline(always)]
pub fn opc_rem_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_rem_w(ctx.a, ctx.b);
}

/* MIN / MAX operations */

/// Sets c to the minimum of a and b as 64-bits unsigned values (and flag to false)
#[inline(always)]
pub const fn op_minu(a: u64, b: u64) -> (u64, bool) {
    if a < b {
        (a, false)
    } else {
        (b, false)
    }
}

/// InstContext-based wrapper over op_minu()
#[inline(always)]
pub fn opc_minu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_minu(ctx.a, ctx.b);
}

/// Sets c to the minimum of a and b as 64-bits signed values (and flag to false)
#[inline(always)]
pub const fn op_min(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) < (b as i64) {
        (a, false)
    } else {
        (b, false)
    }
}

/// InstContext-based wrapper over op_min()
#[inline(always)]
pub fn opc_min(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_min(ctx.a, ctx.b);
}

/// Sets c to the minimum of a and b as 32-bits unsigned values (and flag to false)
#[inline(always)]
pub const fn op_minu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) < (b as u32) {
        (a as i32 as i64 as u64, false)
    } else {
        (b as i32 as i64 as u64, false)
    }
}

/// InstContext-based wrapper over op_minu_w()
#[inline(always)]
pub fn opc_minu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_minu_w(ctx.a, ctx.b);
}

/// Sets c to the minimum of a and b as 32-bits signed values (and flag to false)
#[inline(always)]
pub const fn op_min_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) < (b as i32) {
        (a as i32 as i64 as u64, false)
    } else {
        (b as i32 as i64 as u64, false)
    }
}

/// InstContext-based wrapper over op_min_w()
#[inline(always)]
pub fn opc_min_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_min_w(ctx.a, ctx.b);
}

/// Sets c to the maximum of a and b as 64-bits unsigned values (and flag to false)
#[inline(always)]
pub const fn op_maxu(a: u64, b: u64) -> (u64, bool) {
    if a > b {
        (a, false)
    } else {
        (b, false)
    }
}

/// InstContext-based wrapper over op_maxu()
#[inline(always)]
pub fn opc_maxu(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_maxu(ctx.a, ctx.b);
}

/// Sets c to the maximum of a and b as 64-bits signed values (and flag to false)
#[inline(always)]
pub const fn op_max(a: u64, b: u64) -> (u64, bool) {
    if (a as i64) > (b as i64) {
        (a, false)
    } else {
        (b, false)
    }
}

/// InstContext-based wrapper over op_max()
#[inline(always)]
pub fn opc_max(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_max(ctx.a, ctx.b);
}

/// Sets c to the maximum of a and b as 32-bits unsigned values (and flag to false)
#[inline(always)]
pub const fn op_maxu_w(a: u64, b: u64) -> (u64, bool) {
    if (a as u32) > (b as u32) {
        (a as i32 as i64 as u64, false)
    } else {
        (b as i32 as i64 as u64, false)
    }
}

/// InstContext-based wrapper over op_maxu_w()
#[inline(always)]
pub fn opc_maxu_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_maxu_w(ctx.a, ctx.b);
}

/// Sets c to the maximum of a and b as 32-bits signed values (and flag to false)
#[inline(always)]
pub const fn op_max_w(a: u64, b: u64) -> (u64, bool) {
    if (a as i32) > (b as i32) {
        (a as i32 as i64 as u64, false)
    } else {
        (b as i32 as i64 as u64, false)
    }
}

/// InstContext-based wrapper over op_max_w()
#[inline(always)]
pub fn opc_max_w(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_max_w(ctx.a, ctx.b);
}

/* PRECOMPILED operations */

/// Performs a Keccak-f hash over a 1600-bits input state stored in memory at the address
/// specified by register A0, and stores the output state in the same memory address
#[inline(always)]
pub fn opc_keccak(ctx: &mut InstContext) {
    // Get address from b (a = step)
    let address = ctx.b;
    if address & 0x7 != 0 {
        panic!("opc_keccak() found address not aligned to 8 bytes");
    }

    // Allocate room for 25 u64 = 128 bytes = 1600 bits
    const WORDS: usize = 25;
    let mut data = [0u64; WORDS];

    // Get input data from memory or from the precompiled context
    match ctx.emulation_mode {
        EmulationMode::Mem => {
            // Read data from the memory address
            for (i, d) in data.iter_mut().enumerate() {
                *d = ctx.mem.read(address + (8 * i as u64), 8);
            }

            // Call keccakf
            keccakf(&mut data);

            // Write data to the memory address
            for (i, d) in data.iter().enumerate() {
                ctx.mem.write(address + (8 * i as u64), *d, 8);
            }
        }
        EmulationMode::GenerateMemReads => {
            // Read data from the memory address
            for (i, d) in data.iter_mut().enumerate() {
                *d = ctx.mem.read(address + (8 * i as u64), 8);
            }

            // Copy data to the precompiled context
            ctx.precompiled.input_data.clear();
            for (i, d) in data.iter_mut().enumerate() {
                ctx.precompiled.input_data.push(*d);
            }

            // Call keccakf
            keccakf(&mut data);

            // Write data to the memory address
            for (i, d) in data.iter().enumerate() {
                ctx.mem.write(address + (8 * i as u64), *d, 8);
            }

            // Write data to the precompiled context
            ctx.precompiled.output_data.clear();
            for (i, d) in data.iter_mut().enumerate() {
                ctx.precompiled.output_data.push(*d);
            }
        }
        EmulationMode::ConsumeMemReads => {
            // Check input data has the expected length
            if ctx.precompiled.input_data.len() != WORDS {
                panic!(
                    "opc_keccak() found ctx.precompiled.input_data.len={} != {}",
                    ctx.precompiled.input_data.len(),
                    WORDS
                );
            }
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Keccak can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_keccak(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_keccak() is not implemented");
}

#[inline(always)]
pub fn ops_keccak(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_direct_data(ctx, stats, 25, 25);
}

/// Performs a Sha256-f hash over a 256-bits input state and 512-bits hash state stored in memory at the address
/// specified by register A0, and stores the output state in the same memory address
#[inline(always)]
pub fn opc_sha256(ctx: &mut InstContext) {
    // Allocate room for 12 u64 = 96 bytes = 768 bits (2 extra for indirections)
    const WORDS: usize = 2 + 2 * 4 + 4;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 2, 2, 4, 4, &mut data, "sha256");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // Get the state and input slices
        let (ind, rest) = data.split_at_mut(2);
        let (state_slice, input_slice) = rest.split_at_mut(4);
        let state: &mut [u64; 4] = state_slice.try_into().unwrap();
        let input: &[u64; 8] = input_slice[..8].try_into().unwrap();

        // Compute the sha output with the fastest implementation available
        sha256f(state, input);

        for (i, d) in state.iter().enumerate() {
            ctx.mem.write(ind[0] + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Sha256 can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_sha256(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_sha256() is not implemented");
}

#[inline(always)]
pub fn ops_sha256(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[4, 8], &[], 1);
}

#[inline(always)]
pub fn precompiled_load_data(
    ctx: &mut InstContext,
    params_count: usize,
    load_indirections: usize,
    load_chunks: usize,
    load_rem: usize,
    data: &mut [u64],
    title: &str,
) {
    internal_precompiled_load_data(
        ctx,
        params_count,
        load_indirections,
        load_chunks,
        load_rem,
        0,
        data,
        title,
    );
}

#[inline(always)]
pub fn precompiled_load_data_with_result(
    ctx: &mut InstContext,
    params_count: usize,
    load_indirections: usize,
    load_chunks: usize,
    load_rem: usize,
    data: &mut [u64],
    title: &str,
) {
    internal_precompiled_load_data(
        ctx,
        params_count,
        load_indirections,
        load_chunks,
        load_rem,
        1,
        data,
        title,
    );
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn internal_precompiled_load_data(
    ctx: &mut InstContext,
    params_count: usize,
    load_indirections: usize,
    load_chunks: usize,
    load_rem: usize,
    result: usize,
    data: &mut [u64],
    title: &str,
) {
    let address = ctx.b;
    if address & 0x7 != 0 {
        panic!("precompiled_check_address() found address not aligned to 8 bytes");
    }
    if let EmulationMode::ConsumeMemReads = ctx.emulation_mode {
        // Check input data has the expected length
        let expected_len = params_count + load_indirections * load_chunks + load_rem + result;
        if ctx.precompiled.input_data.len() != expected_len {
            panic!(
                "[{title}] ctx.precompiled.input_data.len={} != {expected_len} [{params_count}+{load_indirections}*{load_chunks}+{load_rem}+{result}]",
                ctx.precompiled.input_data.len(),
            );
        }
        // Read data from the precompiled context
        for (i, d) in data.iter_mut().enumerate() {
            *d = ctx.precompiled.input_data[i];
        }
        // Write the input data address to the precompiled context
        // ctx.precompiled.input_data_address = address;
        return;
    }

    // Write the indirections to data
    for (i, data) in data.iter_mut().enumerate().take(params_count) {
        let indirection = ctx.mem.read(address + (8 * i as u64), 8);
        if address & 0x7 != 0 {
            panic!("precompiled_check_address() found address[{i}] not aligned to 8 bytes");
        }
        *data = indirection;
    }

    let mut data_offset = params_count;
    for i in 0..load_indirections {
        let data_offset = i * load_chunks + data_offset;
        // if there aren't indirections, take directly from the address
        let param_address = if params_count == 0 { address + data_offset as u64 } else { data[i] };
        for j in 0..load_chunks {
            let addr = param_address + (8 * j as u64);
            data[data_offset + j] = ctx.mem.read(addr, 8);
        }
    }

    // Process the remanent of the last chunk
    if load_rem > 0 {
        data_offset += (load_indirections - 1) * load_chunks;
        let param_address = if params_count == 0 {
            address + data_offset as u64
        } else {
            data[load_indirections - 1]
        };
        for j in load_chunks..load_chunks + load_rem {
            let addr = param_address + (8 * j as u64);
            data[data_offset + j] = ctx.mem.read(addr, 8);
        }
    }

    if let EmulationMode::GenerateMemReads = ctx.emulation_mode {
        ctx.precompiled.input_data.clear();
        for (i, d) in data.iter_mut().enumerate() {
            ctx.precompiled.input_data.push(*d);
        }
        ctx.precompiled.step = ctx.step;
    }
}

#[inline(always)]
pub fn precompiled_stats_data(
    ctx: &InstContext,
    stats: &mut dyn OpStats,
    inputs: &[u32],
    outputs: &[u32],
    inputs_reduce_count: usize,
) {
    let param_addr = ctx.b;

    stats.mem_align_read(param_addr, inputs.len() + outputs.len());
    for (index, count) in inputs.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let input_addr = ctx.mem.read(param_addr + (8 * index as u64), 8);
        stats.mem_align_read(input_addr, *count as usize);
        if index < inputs_reduce_count {
            stats.mem_align_write(input_addr, *count as usize);
        }
    }
    let index_offset = inputs.len();
    for (index, count) in outputs.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let output_addr = ctx.mem.read(param_addr + (8 * (index + index_offset) as u64), 8);
        stats.mem_align_write(output_addr, *count as usize);
    }
}

#[inline(always)]
pub fn precompiled_stats_direct_data(
    ctx: &InstContext,
    stats: &mut dyn OpStats,
    inputs: usize,
    outputs: usize,
) {
    let param_addr = ctx.b;

    stats.mem_align_read(param_addr, inputs);
    stats.mem_align_write(param_addr, outputs);
}

pub fn opc_add256(ctx: &mut InstContext) {
    const WORDS: usize = 4 + 1 + 2 * 4;
    let mut data = [0u64; WORDS];

    precompiled_load_data_with_result(ctx, 4, 2, 4, 0, &mut data, "add256");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 3 indirections
        // 0 - addr_a
        // 1 - addr_b
        // 2 - cin
        // 3 - addr_c
        let cin = data[2];
        let (params, rest) = data.split_at(4); // params(4)
        let (a, rest) = rest.split_at(4);
        let (b, _) = rest.split_at(4);

        let a: &[u64; 4] = a.try_into().expect("opc_add256: a.len != 4");
        let b: &[u64; 4] = b.try_into().expect("opc_add256: b.len != 4");
        let mut c = [0u64; 4];
        let cout = precompiles_helpers::add256(a, b, cin, &mut c);

        let c_addr = params[3];
        for (i, c_item) in c.iter().enumerate() {
            ctx.mem.write(c_addr + (8 * i as u64), *c_item, 8);
        }
        if let EmulationMode::GenerateMemReads = ctx.emulation_mode {
            ctx.precompiled.input_data[4 + 2 * 4] = cout;
        }
        ctx.c = cout;
        ctx.flag = cout != 0;
    } else {
        assert!(data[4 + 2 * 4] <= 1, "opc_add256: cout > 1");
        ctx.c = data[4 + 2 * 4];
        ctx.flag = data[4 + 2 * 4] != 0;
    }
}

/// Unimplemented.  Arith256 can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_add256(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_add256() is not implemented");
}

#[inline(always)]
pub fn ops_add256(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[4, 4, 0], &[4], 0);
}

#[inline(always)]
pub fn opc_arith256(ctx: &mut InstContext) {
    const WORDS: usize = 5 + 3 * 4;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 5, 3, 4, 0, &mut data, "arith256");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 5 indirections
        let (_, rest) = data.split_at(5);
        let (a, rest) = rest.split_at(4);
        let (b, c) = rest.split_at(4);

        let a: &[u64; 4] = a.try_into().expect("opc_arith256: a.len != 4");
        let b: &[u64; 4] = b.try_into().expect("opc_arith256: b.len != 4");
        let c: &[u64; 4] = c.try_into().expect("opc_arith256: c.len != 4");

        let mut dl = [0u64; 4];
        let mut dh = [0u64; 4];

        precompiles_helpers::arith256(a, b, c, &mut dl, &mut dh);

        // [a,b,c,3:dl,4:dh]
        for (i, dl_item) in dl.iter().enumerate() {
            ctx.mem.write(data[3] + (8 * i as u64), *dl_item, 8);
        }
        for (i, dh_item) in dh.iter().enumerate() {
            ctx.mem.write(data[4] + (8 * i as u64), *dh_item, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Arith256 can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_arith256(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_arith256() is not implemented");
}

#[inline(always)]
pub fn ops_arith256(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[4, 4, 4], &[4, 4], 0);
}

#[inline(always)]
pub fn opc_arith256_mod(ctx: &mut InstContext) {
    const WORDS: usize = 5 + 4 * 4;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 5, 4, 4, 0, &mut data, "arith256_mod");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 5 indirections
        let (_, rest) = data.split_at(5);
        let (a, rest) = rest.split_at(4);
        let (b, rest) = rest.split_at(4);
        let (c, module) = rest.split_at(4);
        let mut d = [0u64; 4];

        let a: &[u64; 4] = a.try_into().expect("opc_arith256_mod: a.len != 4");
        let b: &[u64; 4] = b.try_into().expect("opc_arith256_mod: b.len != 4");
        let c: &[u64; 4] = c.try_into().expect("opc_arith256_mod: c.len != 4");
        let module: &[u64; 4] = module.try_into().expect("opc_arith256_mod: module.len != 4");

        let mut d = [0u64; 4];

        precompiles_helpers::arith256_mod(a, b, c, module, &mut d);

        // [a,b,c,module,4:d]
        for (i, d) in d.iter().enumerate() {
            ctx.mem.write(data[4] + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Arith256Mod can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_arith256_mod(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_arith256_mod() is not implemented");
}

#[inline(always)]
pub fn ops_arith256_mod(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[4, 4, 4, 4], &[4], 0);
}

#[inline(always)]
pub fn opc_secp256k1_add(ctx: &mut InstContext) {
    const WORDS: usize = 2 + 2 * 8;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 2, 2, 8, 0, &mut data, "secp256k1_add");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 2 indirections
        let (_, rest) = data.split_at(2);
        let (p1, p2) = rest.split_at(8);

        let p1: &[u64; 8] = p1.try_into().expect("opc_secp256k1_add: p1.len != 8");
        let p2: &[u64; 8] = p2.try_into().expect("opc_secp256k1_add: p2.len != 8");
        let mut p3 = [0u64; 8];

        precompiles_helpers::secp256k1_add(p1, p2, &mut p3);

        // [0:p1,p2]
        for (i, d) in p3.iter().enumerate() {
            ctx.mem.write(data[0] + (8 * i as u64), *d, 8);
        }
    }
    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Secp256k1Add can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_secp256k1_add(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_secp256k1_add() is not implemented");
}

#[inline(always)]
pub fn ops_secp256k1_add(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[8, 8], &[], 1);
}

#[inline(always)]
pub fn opc_secp256k1_dbl(ctx: &mut InstContext) {
    const WORDS: usize = 8; // one input of 8 64-bit words
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 0, 1, 8, 0, &mut data, "secp256k1_dbl");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        let p1: &[u64; 8] = &data;
        let mut p3 = [0u64; 8];

        precompiles_helpers::secp256k1_dbl(p1, &mut p3);

        for (i, d) in p3.iter().enumerate() {
            ctx.mem.write(ctx.b + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Secp256k1Dbl can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_secp256k1_dbl(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_secp256k1_dbl() is not implemented");
}

#[inline(always)]
pub fn ops_secp256k1_dbl(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_direct_data(ctx, stats, 8, 8);
}

#[inline(always)]
pub fn opc_bn254_curve_add(ctx: &mut InstContext) {
    const WORDS: usize = 2 + 2 * 8;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 2, 2, 8, 0, &mut data, "bn254_curve_add");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 2 indirections
        let (_, rest) = data.split_at(2);
        let (p1, p2) = rest.split_at(8);

        let p1: &[u64; 8] = p1.try_into().expect("opc_bn254_curve_add: p1.len != 8");
        let p2: &[u64; 8] = p2.try_into().expect("opc_bn254_curve_add: p2.len != 8");
        let mut p3 = [0u64; 8];

        precompiles_helpers::bn254_curve_add(p1, p2, &mut p3);

        // [0:p1,p2]
        for (i, d) in p3.iter().enumerate() {
            ctx.mem.write(data[0] + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Bn254CurveAdd can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_bn254_curve_add(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_bn254_curve_add() is not implemented");
}

#[inline(always)]
pub fn ops_bn254_curve_add(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[8, 8], &[], 1);
}

#[inline(always)]
pub fn opc_bn254_curve_dbl(ctx: &mut InstContext) {
    const WORDS: usize = 8; // one input of 8 64-bit words
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 0, 1, 8, 0, &mut data, "bn254_curve_dbl");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        let p1: &[u64; 8] = &data;
        let mut p3 = [0u64; 8];

        precompiles_helpers::bn254_curve_dbl(p1, &mut p3);

        for (i, d) in p3.iter().enumerate() {
            ctx.mem.write(ctx.b + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Bn254CurveDbl can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_bn254_curve_dbl(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_bn254_curve_dbl() is not implemented");
}

#[inline(always)]
pub fn ops_bn254_curve_dbl(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_direct_data(ctx, stats, 8, 8);
}

#[inline(always)]
pub fn opc_bn254_complex_add(ctx: &mut InstContext) {
    const WORDS: usize = 2 + 2 * 8;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 2, 2, 8, 0, &mut data, "bn254_complex_add");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 2 indirections
        let (_, rest) = data.split_at(2);
        let (f1, f2) = rest.split_at(8);

        let f1: &[u64; 8] = f1.try_into().expect("opc_bn254_complex_add: f1.len != 8");
        let f2: &[u64; 8] = f2.try_into().expect("opc_bn254_complex_add: f2.len != 8");
        let mut f3 = [0u64; 8];

        precompiles_helpers::bn254_complex_add(f1, f2, &mut f3);

        // [0:f1,f2]
        for (i, d) in f3.iter().enumerate() {
            ctx.mem.write(data[0] + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Bn254ComplexAdd can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_bn254_complex_add(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_bn254_complex_add() is not implemented");
}

#[inline(always)]
pub fn ops_bn254_complex_add(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[8, 8], &[], 1);
}

#[inline(always)]
pub fn opc_bn254_complex_sub(ctx: &mut InstContext) {
    const WORDS: usize = 2 + 2 * 8;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 2, 2, 8, 0, &mut data, "bn254_complex_sub");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 2 indirections
        let (_, rest) = data.split_at(2);
        let (f1, f2) = rest.split_at(8);

        let f1: &[u64; 8] = f1.try_into().expect("opc_bn254_complex_sub: f1.len != 8");
        let f2: &[u64; 8] = f2.try_into().expect("opc_bn254_complex_sub: f2.len != 8");
        let mut f3 = [0u64; 8];

        precompiles_helpers::bn254_complex_sub(f1, f2, &mut f3);

        // [0:f1,f2]
        for (i, d) in f3.iter().enumerate() {
            ctx.mem.write(data[0] + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Bn254ComplexSub can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_bn254_complex_sub(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_bn254_complex_sub() is not implemented");
}

#[inline(always)]
pub fn ops_bn254_complex_sub(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[8, 8], &[], 1);
}

#[inline(always)]
pub fn opc_bn254_complex_mul(ctx: &mut InstContext) {
    const WORDS: usize = 2 + 2 * 8;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 2, 2, 8, 0, &mut data, "bn254_complex_mul");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 2 indirections
        let (_, rest) = data.split_at(2);
        let (f1, f2) = rest.split_at(8);

        let f1: &[u64; 8] = f1.try_into().expect("opc_bn254_complex_mul: f1.len != 8");
        let f2: &[u64; 8] = f2.try_into().expect("opc_bn254_complex_mul: f2.len != 8");
        let mut f3 = [0u64; 8];

        precompiles_helpers::bn254_complex_mul(f1, f2, &mut f3);

        // [0:f1,f2]
        for (i, d) in f3.iter().enumerate() {
            ctx.mem.write(data[0] + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Bn254ComplexMul can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_bn254_complex_mul(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_bn254_complex_mul() is not implemented");
}

#[inline(always)]
pub fn ops_bn254_complex_mul(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[8, 8], &[], 1);
}

#[inline(always)]
pub fn opc_arith384_mod(ctx: &mut InstContext) {
    const WORDS: usize = 5 + 4 * 6;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 5, 4, 6, 0, &mut data, "arith384_mod");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 5 indirections
        let (_, rest) = data.split_at(5);
        let (a, rest) = rest.split_at(6);
        let (b, rest) = rest.split_at(6);
        let (c, module) = rest.split_at(6);
        let mut d = [0u64; 6];

        let a: &[u64; 6] = a.try_into().expect("opc_arith384_mod: a.len != 6");
        let b: &[u64; 6] = b.try_into().expect("opc_arith384_mod: b.len != 6");
        let c: &[u64; 6] = c.try_into().expect("opc_arith384_mod: c.len != 6");
        let module: &[u64; 6] = module.try_into().expect("opc_arith384_mod: module.len != 6");

        let mut d = [0u64; 6];

        precompiles_helpers::arith384_mod(a, b, c, module, &mut d);

        // [a,b,c,module,4:d]
        for (i, d) in d.iter().enumerate() {
            ctx.mem.write(data[4] + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Arith384Mod can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_arith384_mod(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_arith384_mod() is not implemented");
}

#[inline(always)]
pub fn ops_arith384_mod(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[6, 6, 6, 6], &[6], 0);
}

#[inline(always)]
pub fn opc_bls12_381_curve_add(ctx: &mut InstContext) {
    const WORDS: usize = 2 + 2 * 12;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 2, 2, 12, 0, &mut data, "bls12_381_curve_add");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 2 indirections
        let (_, rest) = data.split_at(2);
        let (p1, p2) = rest.split_at(12);

        let p1: &[u64; 12] = p1.try_into().expect("opc_bls12_381_curve_add: p1.len != 12");
        let p2: &[u64; 12] = p2.try_into().expect("opc_bls12_381_curve_add: p2.len != 12");
        let mut p3 = [0u64; 12];

        precompiles_helpers::bls12_381_curve_add(p1, p2, &mut p3);

        // [0:p1,p2]
        for (i, d) in p3.iter().enumerate() {
            ctx.mem.write(data[0] + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Bls12_381CurveAdd can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_bls12_381_curve_add(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_bls12_381_curve_add() is not implemented");
}

#[inline(always)]
pub fn ops_bls12_381_curve_add(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[12, 12], &[], 1);
}

#[inline(always)]
pub fn opc_bls12_381_curve_dbl(ctx: &mut InstContext) {
    const WORDS: usize = 12;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 0, 1, 12, 0, &mut data, "bls12_381_curve_dbl");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        let p1: &[u64; 12] = &data;
        let mut p3 = [0u64; 12];

        precompiles_helpers::bls12_381_curve_dbl(p1, &mut p3);

        for (i, d) in p3.iter().enumerate() {
            ctx.mem.write(ctx.b + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Bls12_381CurveDbl can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_bls12_381_curve_dbl(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_bls12_381_curve_dbl() is not implemented");
}

#[inline(always)]
pub fn ops_bls12_381_curve_dbl(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_direct_data(ctx, stats, 12, 12);
}

#[inline(always)]
pub fn opc_bls12_381_complex_add(ctx: &mut InstContext) {
    const WORDS: usize = 2 + 2 * 12;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 2, 2, 12, 0, &mut data, "bls12_381_complex_add");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 2 indirections
        let (_, rest) = data.split_at(2);
        let (f1, f2) = rest.split_at(12);

        let f1: &[u64; 12] = f1.try_into().expect("opc_bls12_381_complex_add: f1.len != 12");
        let f2: &[u64; 12] = f2.try_into().expect("opc_bls12_381_complex_add: f2.len != 12");
        let mut f3 = [0u64; 12];

        precompiles_helpers::bls12_381_complex_add(f1, f2, &mut f3);

        // [0:f1,f2]
        for (i, d) in f3.iter().enumerate() {
            ctx.mem.write(data[0] + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Bls12_381ComplexAdd can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_bls12_381_complex_add(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_bls12_381_complex_add() is not implemented");
}

#[inline(always)]
pub fn ops_bls12_381_complex_add(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[12, 12], &[], 1);
}

#[inline(always)]
pub fn opc_bls12_381_complex_sub(ctx: &mut InstContext) {
    const WORDS: usize = 2 + 2 * 12;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 2, 2, 12, 0, &mut data, "bls12_381_complex_sub");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 2 indirections
        let (_, rest) = data.split_at(2);
        let (f1, f2) = rest.split_at(12);

        let f1: &[u64; 12] = f1.try_into().expect("opc_bls12_381_complex_sub: f1.len != 12");
        let f2: &[u64; 12] = f2.try_into().expect("opc_bls12_381_complex_sub: f2.len != 12");
        let mut f3 = [0u64; 12];

        precompiles_helpers::bls12_381_complex_sub(f1, f2, &mut f3);

        // [0:f1,f2]
        for (i, d) in f3.iter().enumerate() {
            ctx.mem.write(data[0] + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Bls12_381ComplexSub can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_bls12_381_complex_sub(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_bls12_381_complex_sub() is not implemented");
}

#[inline(always)]
pub fn ops_bls12_381_complex_sub(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[12, 12], &[], 1);
}

#[inline(always)]
pub fn opc_bls12_381_complex_mul(ctx: &mut InstContext) {
    const WORDS: usize = 2 + 2 * 12;
    let mut data = [0u64; WORDS];

    precompiled_load_data(ctx, 2, 2, 12, 0, &mut data, "bls12_381_complex_mul");

    if ctx.emulation_mode != EmulationMode::ConsumeMemReads {
        // ignore 2 indirections
        let (_, rest) = data.split_at(2);
        let (f1, f2) = rest.split_at(12);

        let f1: &[u64; 12] = f1.try_into().expect("opc_bls12_381_complex_mul: f1.len != 12");
        let f2: &[u64; 12] = f2.try_into().expect("opc_bls12_381_complex_mul: f2.len != 12");
        let mut f3 = [0u64; 12];

        precompiles_helpers::bls12_381_complex_mul(f1, f2, &mut f3);

        // [0:f1,f2]
        for (i, d) in f3.iter().enumerate() {
            ctx.mem.write(data[0] + (8 * i as u64), *d, 8);
        }
    }

    ctx.c = 0;
    ctx.flag = false;
}

/// Unimplemented.  Bls12_381ComplexMul can only be called from the system call context via InstContext.
/// This is provided just for completeness.
#[inline(always)]
pub fn op_bls12_381_complex_mul(_a: u64, _b: u64) -> (u64, bool) {
    unimplemented!("op_bls12_381_complex_mul() is not implemented");
}

#[inline(always)]
pub fn ops_bls12_381_complex_mul(ctx: &InstContext, stats: &mut dyn OpStats) {
    precompiled_stats_data(ctx, stats, &[12, 12], &[], 1);
}

impl From<ZiskRequiredOperation> for ZiskOp {
    fn from(value: ZiskRequiredOperation) -> Self {
        ZiskOp::try_from_code(value.opcode).unwrap()
    }
}

/// Copies register b into c as a public output data record, where a contains the data index (and
/// sets flag to false)
#[inline(always)]
pub const fn op_pubout(a: u64, b: u64) -> (u64, bool) {
    (b, false)
}

/// InstContext-based wrapper over op_pubout()
#[inline(always)]
pub fn opc_pubout(ctx: &mut InstContext) {
    (ctx.c, ctx.flag) = op_pubout(ctx.a, ctx.b);
    //println!("public ${} = {:#018x}", ctx.a, ctx.b);
}

/// Implements fcall_param, free input data call parameter
#[inline(always)]
pub fn op_fcall_param(a: u64, b: u64) -> (u64, bool) {
    unimplemented!("op_fcall_param() is not implemented");
}

/// InstContext-based wrapper over op_fcall_param()
#[inline(always)]
pub fn opc_fcall_param(ctx: &mut InstContext) {
    // Set c and flag according to the spec
    ctx.c = ctx.b;
    ctx.flag = false;

    // Do nothing when emulating in consume memory reads mode;
    // data will be directly obtained from mem_reads
    if ctx.emulation_mode == EmulationMode::ConsumeMemReads {
        return;
    }

    // Get param size from a
    let words = ctx.a;

    // Get param chunk from b
    let param = ctx.b;

    // Check for consistency
    if (ctx.fcall.parameters_size + words) as usize > FCALL_PARAMS_MAX_SIZE {
        panic!(
            "opc_fcall_param({0}) called with ctx.fcall.parameters_size({1}) + param({0})>{2}",
            words, ctx.fcall.parameters_size, FCALL_PARAMS_MAX_SIZE
        );
    }

    // Store param in context
    if words == 1 {
        ctx.fcall.parameters[ctx.fcall.parameters_size as usize] = param;
        ctx.fcall.parameters_size += 1;
    } else {
        let addr = param;
        for i in 0..words {
            let value = ctx.mem.read(addr + i * 8, 8);
            ctx.fcall.parameters[(ctx.fcall.parameters_size + i) as usize] = value;
        }
        ctx.fcall.parameters_size += words;
    }
}

/// Implements fcall, free input data calls
#[inline(always)]
pub fn op_fcall(a: u64, b: u64) -> (u64, bool) {
    unimplemented!("op_fcall() is not implemented");
}

/// InstContext-based wrapper over op_fcall()
#[inline(always)]
pub fn opc_fcall(ctx: &mut InstContext) {
    // Set c and flag according to the spec
    ctx.c = ctx.b;
    ctx.flag = false;

    // Do nothing when emulating in consume memory reads mode;
    // data will be directly obtained from mem_reads
    if ctx.emulation_mode == EmulationMode::ConsumeMemReads {
        return;
    }

    // Get function id from a
    let function_id = ctx.a;

    let iresult = fcall_proxy(function_id, &ctx.fcall.parameters, &mut ctx.fcall.result);

    if iresult < 0 {
        panic!("opc_fcall() failed calling Fcall() function_id={function_id} iresult={iresult}");
    }

    // Copy result
    if (iresult > 0) {
        ctx.mem.free_input = ctx.fcall.result[0];
    } else {
        ctx.mem.free_input = 0;
    }
    ctx.fcall.result_got = 1;
    ctx.fcall.result_size = iresult as u64;
    ctx.fcall.parameters_size = 0;
}

/// Implements fcall_get, fcall result
#[inline(always)]
pub fn op_fcall_get(a: u64, b: u64) -> (u64, bool) {
    unimplemented!("op_fcall_get() is not implemented");
}

/// InstContext-based wrapper over op_fcall_get()
#[inline(always)]
pub fn opc_fcall_get(ctx: &mut InstContext) {
    ctx.c = ctx.b;
    ctx.flag = false;

    // Do nothing when emulating in consume memory reads mode;
    // data will be directly obtained from mem_reads
    if ctx.emulation_mode == EmulationMode::ConsumeMemReads {
        return;
    }
    // Check for consistency
    if ctx.fcall.result_size == 0 {
        panic!("opc_fcall_get() called with ctx.fcall.result_size==0");
    }
    if ctx.fcall.result_size as usize > FCALL_RESULT_MAX_SIZE {
        panic!("opc_fcall_get() called with ctx.fcall.result_size=={}>32", ctx.fcall.result_size);
    }
    if ctx.fcall.result_got > ctx.fcall.result_size {
        panic!(
            "opc_fcall_get() called with ctx.fcall.result_got({}) >= ctx.fcall.result_size {}",
            ctx.fcall.result_got, ctx.fcall.result_size
        );
    }

    // Copy the data into c and advance counter
    if ctx.fcall.result_got >= ctx.fcall.result_size {
        ctx.mem.free_input = 0;
    } else {
        ctx.mem.free_input = ctx.fcall.result[ctx.fcall.result_got as usize];
    }
    ctx.fcall.result_got += 1;
    ctx.flag = false;
}

/// Implements halt
#[inline(always)]
pub fn op_halt(a: u64, b: u64) -> (u64, bool) {
    unimplemented!("op_halt() is not implemented");
}

/// InstContext-based wrapper over op_halt()
#[inline(always)]
pub fn opc_halt(ctx: &mut InstContext) {
    ctx.error = true;
    ctx.c = 0;
    ctx.flag = false;
}
