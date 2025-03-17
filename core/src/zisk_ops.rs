//! * Defines the operations that can be executed in Zisk as part of an instruction.
//! * The macro `define_ops` is used to define every operation, including its opcode, human-readable
//!   name, type, etc.
//! * The opcode operation functions are called `op_<opcode>`, they accept 2 input parameters a and
//!   b, and return 2 output results c and flag.
//! * The `opc_<opcode>` functions are wrappers over the `op_<opcode>` functions that accept an
//!   `InstContext` (instruction context) as input/output parameter, containg a, b, c and flag
//!   attributes.

#![allow(unused)]

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    num::Wrapping,
    str::FromStr,
};
use tiny_keccak::keccakf;

use crate::{
    InstContext, Mem, PrecompiledEmulationMode, ZiskOperationType, ZiskRequiredOperation, M64,
    REG_A0, SYS_ADDR,
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
    PubOut,
}

impl From<OpType> for ZiskOperationType {
    fn from(op_type: OpType) -> Self {
        match op_type {
            OpType::Internal => ZiskOperationType::Internal,
            OpType::Arith | OpType::ArithA32 | OpType::ArithAm32 => ZiskOperationType::Arith,
            OpType::Binary => ZiskOperationType::Binary,
            OpType::BinaryE => ZiskOperationType::BinaryE,
            OpType::Keccak => ZiskOperationType::Keccak,
            OpType::PubOut => ZiskOperationType::PubOut,
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
            Self::PubOut => write!(f, "PubOut"),
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
    ( $( ($name:ident, $str_name:expr, $type:ident, $steps:expr, $code:expr, $input_size:expr, $call_fn:ident, $call_ab_fn:ident) ),* $(,)? ) => {
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

// Cost definitions
const BINARY_COST: u64 = 75;
const BINARY_E_COST: u64 = 54;
const ARITHA32_COST: u64 = 95;
const ARITHAM32_COST: u64 = 95;
const KECCAK_COST: u64 = 137221;

/// Table of Zisk opcode definitions: enum, name, type, cost, code and implementation functions
/// This table is the backbone of the Zisk processor, it determines what functionality is supported,
/// and what state machine is responsible of proving the execution of every opcode, based on its
/// type.
define_ops! {
    (Flag, "flag", Internal, 0, 0x00, 0, opc_flag, op_flag),
    (CopyB, "copyb", Internal, 0, 0x01, 0, opc_copyb, op_copyb),
    (SignExtendB, "signextend_b", BinaryE, BINARY_E_COST, 0x37, 0, opc_signextend_b, op_signextend_b),
    (SignExtendH, "signextend_h", BinaryE, BINARY_E_COST, 0x38, 0, opc_signextend_h, op_signextend_h),
    (SignExtendW, "signextend_w", BinaryE, BINARY_E_COST, 0x39, 0, opc_signextend_w, op_signextend_w),
    (Add, "add", Binary, BINARY_COST, 0x0c, 0, opc_add, op_add),
    (AddW, "add_w", Binary, BINARY_COST, 0x2c, 0, opc_add_w, op_add_w),
    (Sub, "sub", Binary, BINARY_COST, 0x0d, 0, opc_sub, op_sub),
    (SubW, "sub_w", Binary, BINARY_COST, 0x2d, 0, opc_sub_w, op_sub_w),
    (Sll, "sll", BinaryE, BINARY_E_COST, 0x31, 0, opc_sll, op_sll),
    (SllW, "sll_w", BinaryE, BINARY_E_COST, 0x34, 0, opc_sll_w, op_sll_w),
    (Sra, "sra", BinaryE, BINARY_E_COST, 0x33, 0, opc_sra, op_sra),
    (Srl, "srl", BinaryE, BINARY_E_COST, 0x32, 0, opc_srl, op_srl),
    (SraW, "sra_w", BinaryE, BINARY_E_COST, 0x36, 0, opc_sra_w, op_sra_w),
    (SrlW, "srl_w", BinaryE, BINARY_E_COST, 0x35, 0, opc_srl_w, op_srl_w),
    (Eq, "eq", Binary, BINARY_COST, 0x0b, 0, opc_eq, op_eq),
    (EqW, "eq_w", Binary, BINARY_COST, 0x2b, 0, opc_eq_w, op_eq_w),
    (Ltu, "ltu", Binary, BINARY_COST, 0x08, 0, opc_ltu, op_ltu),
    (Lt, "lt", Binary, BINARY_COST, 0x09, 0, opc_lt, op_lt),
    (LtuW, "ltu_w", Binary, BINARY_COST, 0x28, 0, opc_ltu_w, op_ltu_w),
    (LtW, "lt_w", Binary, BINARY_COST, 0x29, 0, opc_lt_w, op_lt_w),
    (Leu, "leu", Binary, BINARY_COST, 0x0e, 0, opc_leu, op_leu),
    (Le, "le", Binary, BINARY_COST, 0x0f, 0, opc_le, op_le),
    (LeuW, "leu_w", Binary, BINARY_COST, 0x2e, 0, opc_leu_w, op_leu_w),
    (LeW, "le_w", Binary, BINARY_COST, 0x2f, 0, opc_le_w, op_le_w),
    (And, "and", Binary, BINARY_COST, 0x10, 0, opc_and, op_and),
    (Or, "or", Binary, BINARY_COST, 0x11, 0, opc_or, op_or),
    (Xor, "xor", Binary, BINARY_COST, 0x12, 0, opc_xor, op_xor),
    (Mulu, "mulu", ArithAm32, ARITHAM32_COST, 0xb0, 0, opc_mulu, op_mulu),
    (Muluh, "muluh", ArithAm32, ARITHAM32_COST, 0xb1, 0, opc_muluh, op_muluh),
    (Mulsuh, "mulsuh", ArithAm32, ARITHAM32_COST, 0xb3, 0, opc_mulsuh, op_mulsuh),
    (Mul, "mul", ArithAm32, ARITHAM32_COST, 0xb4, 0, opc_mul, op_mul),
    (Mulh, "mulh", ArithAm32, ARITHAM32_COST, 0xb5, 0, opc_mulh, op_mulh),
    (MulW, "mul_w", ArithAm32, ARITHAM32_COST, 0xb6, 0, opc_mul_w, op_mul_w),
    (Divu, "divu", ArithAm32, ARITHAM32_COST, 0xb8, 0, opc_divu, op_divu),
    (Remu, "remu", ArithAm32, ARITHAM32_COST, 0xb9, 0, opc_remu, op_remu),
    (Div, "div", ArithAm32, ARITHAM32_COST, 0xba, 0, opc_div, op_div),
    (Rem, "rem", ArithAm32, ARITHAM32_COST, 0xbb, 0, opc_rem, op_rem),
    (DivuW, "divu_w", ArithA32, ARITHA32_COST, 0xbc, 0, opc_divu_w, op_divu_w),
    (RemuW, "remu_w", ArithA32, ARITHA32_COST, 0xbd, 0, opc_remu_w, op_remu_w),
    (DivW, "div_w", ArithA32, ARITHA32_COST, 0xbe, 0, opc_div_w, op_div_w),
    (RemW, "rem_w", ArithA32, ARITHA32_COST, 0xbf, 0, opc_rem_w, op_rem_w),
    (Minu, "minu", Binary, BINARY_COST, 0x02, 0, opc_minu, op_minu),
    (Min, "min", Binary, BINARY_COST, 0x03, 0, opc_min, op_min),
    (MinuW, "minu_w", Binary, BINARY_COST, 0x22, 0, opc_minu_w, op_minu_w),
    (MinW, "min_w", Binary, BINARY_COST, 0x23, 0, opc_min_w, op_min_w),
    (Maxu, "maxu", Binary, BINARY_COST, 0x04, 0, opc_maxu, op_maxu),
    (Max, "max", Binary, BINARY_COST, 0x05, 0, opc_max, op_max),
    (MaxuW, "maxu_w", Binary, BINARY_COST, 0x24, 0, opc_maxu_w, op_maxu_w),
    (MaxW, "max_w", Binary, BINARY_COST, 0x25, 0, opc_max_w, op_max_w),
    (Keccak, "keccak", Keccak, KECCAK_COST, 0xf1, 200, opc_keccak, op_keccak),
    (PubOut, "pubout", PubOut, 0, 0x30, 0, opc_pubout, op_pubout),
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
    // Get address from register a0 = x10
    let address = ctx.regs[Mem::address_to_register_index(REG_A0)];
    if address & 0x7 != 0 {
        panic!("opc_keccak() found address not aligned to 8 bytes");
    }

    // Allocate room for 25 u64 = 128 bytes = 1600 bits
    const WORDS: usize = 25;
    let mut data = [0u64; WORDS];

    // Get input data from memory or from the precompiled context
    match ctx.precompiled.emulation_mode {
        PrecompiledEmulationMode::None => {
            // Read data from the memory address
            for (i, d) in data.iter_mut().enumerate() {
                *d = ctx.mem.read(address + (8 * i as u64), 8);
            }
        }
        PrecompiledEmulationMode::GenerateMemReads => {
            // Read data from the memory address
            for (i, d) in data.iter_mut().enumerate() {
                *d = ctx.mem.read(address + (8 * i as u64), 8);
            }
            // Copy data to the precompiled context
            ctx.precompiled.input_data.clear();
            for (i, d) in data.iter_mut().enumerate() {
                ctx.precompiled.input_data.push(*d);
            }
            // Write the input data address to the precompiled context
            ctx.precompiled.input_data_address = address;
        }
        PrecompiledEmulationMode::ConsumeMemReads => {
            // Check input data has the expected length
            if ctx.precompiled.input_data.len() != WORDS {
                panic!(
                    "opc_keccak() found ctx.precompiled.input_data.len={} != {}",
                    ctx.precompiled.input_data.len(),
                    WORDS
                );
            }
            // Read data from the precompiled context
            for (i, d) in data.iter_mut().enumerate() {
                *d = ctx.precompiled.input_data[i];
            }
            // Write the input data address to the precompiled context
            ctx.precompiled.input_data_address = address;
        }
    }

    // Call keccakf
    keccakf(&mut data);

    // Write data to the memory address
    for (i, d) in data.iter().enumerate() {
        ctx.mem.write(address + (8 * i as u64), *d, 8);
    }

    // Set input data to the precompiled context
    match ctx.precompiled.emulation_mode {
        PrecompiledEmulationMode::None => {}
        PrecompiledEmulationMode::GenerateMemReads => {
            // Write data to the precompiled context
            ctx.precompiled.output_data.clear();
            for (i, d) in data.iter_mut().enumerate() {
                ctx.precompiled.output_data.push(*d);
            }
            // Write the input data address to the precompiled context
            ctx.precompiled.output_data_address = address;
        }
        PrecompiledEmulationMode::ConsumeMemReads => {}
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
