//! Defines the instructions that can be executed in Zisk

#![allow(unused)]

use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use crate::{zisk_operations::*, InstContext};

/// Determines the type of a [`ZiskOp`]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OpType {
    Internal,
    Arith,
    ArithA32,
    ArithAm32,
    Binary,
    BinaryE,
    Keccak,
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

/// Internal macro used to define all ops in the [`ZiskOp`] enum
macro_rules! define_ops {
    ( $( ($name:ident, $str_name:expr, $type:ident, $steps:expr, $code:expr, $call_fn:ident, $call_ab_fn:ident) ),* $(,)? ) => {
		/// Represents an operation that can be executed in Zisk.
		///
		/// All relevant metadata associated with the operation can be efficiently accessed via
		/// the const methods on this enum.
        #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
        pub enum ZiskOp {
            $(
                $name,
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

			/// Executes the operation on the given [`InstContext`]
			#[inline(always)]
            pub fn call(&self, ctx: &mut InstContext) {
                match self {
                    $(
                        Self::$name => $call_fn(ctx),
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

define_ops! {
    (Flag, "flag", Internal, 0, 0x00, opc_flag, op_flag),
    (CopyB, "copyb", Internal, 0, 0x01, opc_copyb, op_copyb),
    (SignExtendB, "signextend_b", BinaryE, 109, 0x24, opc_signextend_b, op_signextend_b),
    (SignExtendH, "signextend_h", BinaryE, 109, 0x25, opc_signextend_h, op_signextend_h),
    (SignExtendW, "signextend_w", BinaryE, 109, 0x26, opc_signextend_w, op_signextend_w),
    (Add, "add", Binary, 77, 0x02, opc_add, op_add),
    (AddW, "add_w", Binary, 77, 0x12, opc_add_w, op_add_w),
    (Sub, "sub", Binary, 77, 0x03, opc_sub, op_sub),
    (SubW, "sub_w", Binary, 77, 0x13, opc_sub_w, op_sub_w),
    (Sll, "sll", BinaryE, 109, 0x0d, opc_sll, op_sll),
    (SllW, "sll_w", BinaryE, 109, 0x1d, opc_sll_w, op_sll_w),
    (Sra, "sra", BinaryE, 109, 0x0f, opc_sra, op_sra),
    (Srl, "srl", BinaryE, 109, 0x0e, opc_srl, op_srl),
    (SraW, "sra_w", BinaryE, 109, 0x1f, opc_sra_w, op_sra_w),
    (SrlW, "srl_w", BinaryE, 109, 0x1e, opc_srl_w, op_srl_w),
    (Eq, "eq", Binary, 77, 0x08, opc_eq, op_eq),
    (EqW, "eq_w", Binary, 77, 0x18, opc_eq_w, op_eq_w),
    (Ltu, "ltu", Binary, 77, 0x04, opc_ltu, op_ltu),
    (Lt, "lt", Binary, 77, 0x05, opc_lt, op_lt),
    (LtuW, "ltu_w", Binary, 77, 0x14, opc_ltu_w, op_ltu_w),
    (LtW, "lt_w", Binary, 77, 0x15, opc_lt_w, op_lt_w),
    (Leu, "leu", Binary, 77, 0x06, opc_leu, op_leu),
    (Le, "le", Binary, 77, 0x07, opc_le, op_le),
    (LeuW, "leu_w", Binary, 77, 0x16, opc_leu_w, op_leu_w),
    (LeW, "le_w", Binary, 77, 0x17, opc_le_w, op_le_w),
    (And, "and", Binary, 77, 0x20, opc_and, op_and),
    (Or, "or", Binary, 77, 0x21, opc_or, op_or),
    (Xor, "xor", Binary, 77, 0x22, opc_xor, op_xor),
    (Mulu, "mulu", ArithAm32, 97, 0xb0, opc_mulu, op_mulu),
    (Mul, "mul", ArithAm32, 97, 0xb1, opc_mul, op_mul),
    (MulW, "mul_w", ArithAm32, 44, 0xb5, opc_mul_w, op_mul_w),
    (Muluh, "muluh", ArithAm32, 97, 0xb8, opc_muluh, op_muluh),
    (Mulh, "mulh", ArithAm32, 97, 0xb9, opc_mulh, op_mulh),
    (Mulsuh, "mulsuh", ArithAm32, 97, 0xbb, opc_mulsuh, op_mulsuh),
    (Divu, "divu", ArithAm32, 174, 0xc0, opc_divu, op_divu),
    (Div, "div", ArithAm32, 174, 0xc1, opc_div, op_div),
    (DivuW, "divu_w", ArithA32, 136, 0xc4, opc_divu_w, op_divu_w),
    (DivW, "div_w", ArithA32, 136, 0xc5, opc_div_w, op_div_w),
    (Remu, "remu", ArithAm32, 174, 0xc8, opc_remu, op_remu),
    (Rem, "rem", ArithAm32, 174, 0xc9, opc_rem, op_rem),
    (RemuW, "remu_w", ArithA32, 136, 0xcc, opc_remu_w, op_remu_w),
    (RemW, "rem_w", ArithA32, 136, 0xcd, opc_rem_w, op_rem_w),
    (Minu, "minu", Binary, 77, 0x09, opc_minu, op_minu),
    (Min, "min", Binary, 77, 0x0a, opc_min, op_min),
    (MinuW, "minu_w", Binary, 77, 0x19, opc_minu_w, op_minu_w),
    (MinW, "min_w", Binary, 77, 0x1a, opc_min_w, op_min_w),
    (Maxu, "maxu", Binary, 77, 0x0b, opc_maxu, op_maxu),
    (Max, "max", Binary, 77, 0x0c, opc_max, op_max),
    (MaxuW, "maxu_w", Binary, 77, 0x1b, opc_maxu_w, op_maxu_w),
    (MaxW, "max_w", Binary, 77, 0x1c, opc_max_w, op_max_w),
    (Keccak, "keccak", Keccak, 77, 0xf1, opc_keccak, op_keccak),
}
