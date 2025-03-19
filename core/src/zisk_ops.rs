//! * Defines the operations that can be executed in Zisk as part of an instruction.
//! * The macro `define_ops` is used to define every operation, including its opcode, human-readable
//!   name, type, etc.
//! * The opcode operation functions are called `op_<opcode>`, they accept 2 input parameters a and
//!   b, and return 2 output results c and flag.
//! * The `opc_<opcode>` functions are wrappers over the `op_<opcode>` functions that accept an
//!   `InstContext` (instruction context) as input/output parameter, containg a, b, c and flag
//!   attributes.

use crate::{InstContext, Mem, ZiskOperationType, ZiskRequiredOperation, M64};
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Debug, Display},
    num::Wrapping,
    rc::Rc,
    str::FromStr,
};
use zisk_common::{MemPrecompilesOps, ZiskPrecompile};

pub const KECCAK_CODE: u8 = 0xf1;

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
/// Table of Zisk opcode definitions: enum, name, type, cost, code and implementation functions
/// This table is the backbone of the Zisk processor, it determines what functionality is supported,
/// and what state machine is responsible of proving the execution of every opcode, based on its
/// type.
macro_rules! define_ops {
    ( $( ($name:ident, $str_name:expr, $precompile:expr, $type:ident, $steps:expr, $code:expr, $call_fn:ident, $call_ab_fn:ident) ),* $(,)? ) => {
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

			/// Executes the operation on the given [`InstContext`]
			#[inline(always)]
            pub fn call(&self, ctx: &mut InstContext, get_mem_read: Option<&mut dyn FnMut() -> u64>, push_mem_read: Option<&mut dyn FnMut(u64)>, precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>
        ) {
                match self {
                    $(
                        Self::$name => $call_fn(ctx, get_mem_read, push_mem_read, precompiles),
                    )*
                }
            }

            /// Returns the call function of the operation
            pub const fn get_call_function(&self) -> fn(&mut InstContext, Option<&mut dyn FnMut() -> u64>, Option<&mut dyn FnMut(u64)>, Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>) -> () {
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

define_ops! {
    (Flag, "flag", false, Internal, 0, 0x00, opc_flag, op_flag),
    (CopyB, "copyb", false, Internal, 0, 0x01, opc_copyb, op_copyb),
    (SignExtendB, "signextend_b", false, BinaryE, BINARY_E_COST, 0x37, opc_signextend_b, op_signextend_b),
    (SignExtendH, "signextend_h", false, BinaryE, BINARY_E_COST, 0x38, opc_signextend_h, op_signextend_h),
    (SignExtendW, "signextend_w", false, BinaryE, BINARY_E_COST, 0x39, opc_signextend_w, op_signextend_w),
    (Add, "add", false, Binary, BINARY_COST, 0x0c, opc_add, op_add),
    (AddW, "add_w", false, Binary, BINARY_COST, 0x2c, opc_add_w, op_add_w),
    (Sub, "sub", false, Binary, BINARY_COST, 0x0d, opc_sub, op_sub),
    (SubW, "sub_w", false, Binary, BINARY_COST, 0x2d, opc_sub_w, op_sub_w),
    (Sll, "sll", false, BinaryE, BINARY_E_COST, 0x31, opc_sll, op_sll),
    (SllW, "sll_w", false, BinaryE, BINARY_E_COST, 0x34, opc_sll_w, op_sll_w),
    (Sra, "sra", false, BinaryE, BINARY_E_COST, 0x33, opc_sra, op_sra),
    (Srl, "srl", false, BinaryE, BINARY_E_COST, 0x32, opc_srl, op_srl),
    (SraW, "sra_w", false, BinaryE, BINARY_E_COST, 0x36, opc_sra_w, op_sra_w),
    (SrlW, "srl_w", false, BinaryE, BINARY_E_COST, 0x35, opc_srl_w, op_srl_w),
    (Eq, "eq", false, Binary, BINARY_COST, 0x0b, opc_eq, op_eq),
    (EqW, "eq_w", false, Binary, BINARY_COST, 0x2b, opc_eq_w, op_eq_w),
    (Ltu, "ltu", false, Binary, BINARY_COST, 0x08, opc_ltu, op_ltu),
    (Lt, "lt", false, Binary, BINARY_COST, 0x09, opc_lt, op_lt),
    (LtuW, "ltu_w", false, Binary, BINARY_COST, 0x28, opc_ltu_w, op_ltu_w),
    (LtW, "lt_w", false, Binary, BINARY_COST, 0x29, opc_lt_w, op_lt_w),
    (Leu, "leu", false, Binary, BINARY_COST, 0x0e, opc_leu, op_leu),
    (Le, "le", false, Binary, BINARY_COST, 0x0f, opc_le, op_le),
    (LeuW, "leu_w", false, Binary, BINARY_COST, 0x2e, opc_leu_w, op_leu_w),
    (LeW, "le_w", false, Binary, BINARY_COST, 0x2f, opc_le_w, op_le_w),
    (And, "and", false, Binary, BINARY_COST, 0x10, opc_and, op_and),
    (Or, "or", false, Binary, BINARY_COST, 0x11, opc_or, op_or),
    (Xor, "xor", false, Binary, BINARY_COST, 0x12, opc_xor, op_xor),
    (Mulu, "mulu", false, ArithAm32, ARITHAM32_COST, 0xb0, opc_mulu, op_mulu),
    (Muluh, "muluh", false, ArithAm32, ARITHAM32_COST, 0xb1, opc_muluh, op_muluh),
    (Mulsuh, "mulsuh", false, ArithAm32, ARITHAM32_COST, 0xb3, opc_mulsuh, op_mulsuh),
    (Mul, "mul", false, ArithAm32, ARITHAM32_COST, 0xb4, opc_mul, op_mul),
    (Mulh, "mulh", false, ArithAm32, ARITHAM32_COST, 0xb5, opc_mulh, op_mulh),
    (MulW, "mul_w", false, ArithAm32, ARITHAM32_COST, 0xb6, opc_mul_w, op_mul_w),
    (Divu, "divu", false, ArithAm32, ARITHAM32_COST, 0xb8, opc_divu, op_divu),
    (Remu, "remu", false, ArithAm32, ARITHAM32_COST, 0xb9, opc_remu, op_remu),
    (Div, "div", false, ArithAm32, ARITHAM32_COST, 0xba, opc_div, op_div),
    (Rem, "rem", false, ArithAm32, ARITHAM32_COST, 0xbb, opc_rem, op_rem),
    (DivuW, "divu_w", false, ArithA32, ARITHA32_COST, 0xbc, opc_divu_w, op_divu_w),
    (RemuW, "remu_w", false, ArithA32, ARITHA32_COST, 0xbd, opc_remu_w, op_remu_w),
    (DivW, "div_w", false, ArithA32, ARITHA32_COST, 0xbe, opc_div_w, op_div_w),
    (RemW, "rem_w", false, ArithA32, ARITHA32_COST, 0xbf, opc_rem_w, op_rem_w),
    (Minu, "minu", false, Binary, BINARY_COST, 0x02, opc_minu, op_minu),
    (Min, "min", false, Binary, BINARY_COST, 0x03, opc_min, op_min),
    (MinuW, "minu_w", false, Binary, BINARY_COST, 0x22, opc_minu_w, op_minu_w),
    (MinW, "min_w", false, Binary, BINARY_COST, 0x23, opc_min_w, op_min_w),
    (Maxu, "maxu", false, Binary, BINARY_COST, 0x04, opc_maxu, op_maxu),
    (Max, "max", false, Binary, BINARY_COST, 0x05, opc_max, op_max),
    (MaxuW, "maxu_w", false, Binary, BINARY_COST, 0x24, opc_maxu_w, op_maxu_w),
    (MaxW, "max_w", false, Binary, BINARY_COST, 0x25, opc_max_w, op_max_w),
    (Keccak, "keccak", true, Keccak, KECCAK_COST, 0xf1, opc_keccak, op_keccak),
    (PubOut, "pubout", false, PubOut, 0, 0x30, opc_pubout, op_pubout),
}

/* INTERNAL operations */

/// Sets flag to true (and c to 0)
#[inline(always)]
pub const fn op_flag(_a: u64, _b: u64) -> (u64, bool) {
    (0, true)
}

/// InstContext-based wrapper over op_flag()
#[inline(always)]
pub fn opc_flag(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_flag(ctx.a, ctx.b);
}

/// Copies register b into c (and flag to false)
#[inline(always)]
pub const fn op_copyb(_a: u64, b: u64) -> (u64, bool) {
    (b, false)
}

/// InstContext-based wrapper over op_copyb()
#[inline(always)]
pub fn opc_copyb(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_signextend_b(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_signextend_h(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_signextend_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_add(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_add(ctx.a, ctx.b);
}

/// Adds a and b as 32-bit signed values, and stores the result in c (and flag to false)
#[inline(always)]
pub fn op_add_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) + Wrapping(b as i32)).0 as u64, false)
}

/// InstContext-based wrapper over op_add_w()
#[inline(always)]
pub fn opc_add_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_add_w(ctx.a, ctx.b);
}

/// Subtracts a and b as 64-bit unsigned values, and stores the result in c (and sets flag to false)
#[inline(always)]
pub fn op_sub(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a) - Wrapping(b)).0, false)
}

/// InstContext-based wrapper over op_sub()
#[inline(always)]
pub fn opc_sub(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_sub(ctx.a, ctx.b);
}

/// Subtracts a and b as 32-bit signed values, and stores the result in c (and sets flag to false)
#[inline(always)]
pub fn op_sub_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) - Wrapping(b as i32)).0 as u64, false)
}

/// InstContext-based wrapper over op_sub_w()
#[inline(always)]
pub fn opc_sub_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_sll(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_sll_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_sra(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_srl(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_sra_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_srl_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_eq(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_eq_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_ltu(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_lt(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_ltu_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_lt_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_leu(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_le(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_leu_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_le_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_and(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_and(ctx.a, ctx.b);
}

/// Sets c to a OR b, and flag to false
#[inline(always)]
pub const fn op_or(a: u64, b: u64) -> (u64, bool) {
    (a | b, false)
}

/// InstContext-based wrapper over op_or()
#[inline(always)]
pub fn opc_or(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_or(ctx.a, ctx.b);
}

/// Sets c to a XOR b, and flag to false
#[inline(always)]
pub const fn op_xor(a: u64, b: u64) -> (u64, bool) {
    (a ^ b, false)
}

/// InstContext-based wrapper over op_xor()
#[inline(always)]
pub fn opc_xor(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_mulu(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_mulu(ctx.a, ctx.b);
}

/// Sets c to a x b, as 64-bits signed values, and flag to false
#[inline(always)]
pub fn op_mul(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i64) * Wrapping(b as i64)).0 as u64, false)
}

/// InstContext-based wrapper over op_mul()
#[inline(always)]
pub fn opc_mul(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_mul(ctx.a, ctx.b);
}

/// Sets c to a x b, as 32-bits signed values, and flag to false
#[inline(always)]
pub fn op_mul_w(a: u64, b: u64) -> (u64, bool) {
    ((Wrapping(a as i32) * Wrapping(b as i32)).0 as u64, false)
}

/// InstContext-based wrapper over op_mul_w()
#[inline(always)]
pub fn opc_mul_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_mul_w(ctx.a, ctx.b);
}

/// Sets c to the highest 64-bits of a x b, as 128-bits unsigned values, and flag to false
#[inline(always)]
pub const fn op_muluh(a: u64, b: u64) -> (u64, bool) {
    (((a as u128 * b as u128) >> 64) as u64, false)
}

/// InstContext-based wrapper over op_muluh()
#[inline(always)]
pub fn opc_muluh(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_muluh(ctx.a, ctx.b);
}

/// Sets c to the highest 64-bits of a x b, as 128-bits unsigned values, and flag to false
#[inline(always)]
pub const fn op_mulh(a: u64, b: u64) -> (u64, bool) {
    (((((a as i64) as i128) * ((b as i64) as i128)) >> 64) as u64, false)
}

/// InstContext-based wrapper over op_mulh()
#[inline(always)]
pub fn opc_mulh(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_mulh(ctx.a, ctx.b);
}

/// Sets c to the highest 64-bits of a x b, as 128-bits signed values, and flag to false
#[inline(always)]
pub const fn op_mulsuh(a: u64, b: u64) -> (u64, bool) {
    (((((a as i64) as i128) * (b as i128)) >> 64) as u64, false)
}

/// InstContext-based wrapper over op_mulsuh()
#[inline(always)]
pub fn opc_mulsuh(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_divu(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_div(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_divu_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_div_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_remu(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_rem(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_remu_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_rem_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_minu(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_min(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_minu_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_min_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_maxu(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_max(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_maxu_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
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
pub fn opc_max_w(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_max_w(ctx.a, ctx.b);
}

/* PRECOMPILED operations */

/// Performs a Keccak-f hash over a 1600-bits input state stored in memory at the address
/// specified by register A0, and stores the output state in the same memory address
#[inline(always)]
pub fn opc_keccak(
    ctx: &mut InstContext,
    get_mem_read: Option<&mut dyn FnMut() -> u64>,
    mut push_mem_read: Option<&mut dyn FnMut(u64)>,
    precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    let precompiles = precompiles.expect("Precompiles are required for opc_keccak");
    let precompile =
        precompiles.get(&(KECCAK_CODE as usize)).expect("Keccak precompile does not exist");

    let emulation_mode = ctx.precompiled.emulation_mode.clone();
    let mem = Rc::new(RefCell::new(&mut ctx.mem));

    let read_reg_fn = |reg: u64| -> u64 { ctx.regs[Mem::address_to_register_index(reg)] };
    let read_mem_fn = |address: u64, generate_mem_read: bool| -> u64 {
        let value = mem.borrow().read(address, 8);
        if generate_mem_read {
            push_mem_read.as_mut().unwrap()(value);
        }
        value
    };
    let write_mem_fn = |address: u64, value: u64| {
        mem.borrow_mut().write(address, value, 8);
    };
    let write_input_data = |input_data: Vec<u64>| {
        ctx.precompiled.input_data = input_data;
    };

    let get_mem_read = get_mem_read.map(|f| Box::new(f) as Box<dyn FnMut() -> u64>);

    let mem_ops = MemPrecompilesOps::new(
        get_mem_read,
        Box::new(read_reg_fn),
        Box::new(read_mem_fn),
        Box::new(write_mem_fn),
        Box::new(write_input_data),
    );

    let (c, flag) = precompile.execute(0, 0, emulation_mode, mem_ops);

    ctx.c = c;
    ctx.flag = flag;
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
pub const fn op_pubout(_a: u64, b: u64) -> (u64, bool) {
    (b, false)
}

/// InstContext-based wrapper over op_pubout()
#[inline(always)]
pub fn opc_pubout(
    ctx: &mut InstContext,
    _get_mem_read: Option<&mut dyn FnMut() -> u64>,
    _push_mem_read: Option<&mut dyn FnMut(u64)>,
    _precompiles: Option<&HashMap<usize, Box<dyn ZiskPrecompile>>>,
) {
    (ctx.c, ctx.flag) = op_pubout(ctx.a, ctx.b);
    //println!("public ${} = {:#018x}", ctx.a, ctx.b);
}
