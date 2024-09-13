//! Defines the instructions that can be executed in Zisk

#![allow(unused)]

use std::fmt::Debug;

use crate::{zisk_operations::*, InstContext};

/// Determines the type of a [`ZiskOp`]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OpType {
    Internal,
    ArithA32,
    ArithAm32,
    Binary,
    BinaryE,
    Keccak,
}

#[derive(Copy, Clone, Debug)]
pub struct InvalidNameError;

#[derive(Copy, Clone, Debug)]
pub struct InvalidCodeError;

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
            pub fn call(&self, ctx: &mut InstContext) {
                match self {
                    $(
                        Self::$name => $call_fn(ctx),
                    )*
                }
            }

			/// Executes the operation on the given inputs `a` and `b`
            pub const fn call_ab(&self, a: u64, b: u64) -> (u64, bool) {
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
        }
    };
}

define_ops! {
    (Flag, "flag", Internal, 0, 0, opc_flag, op_flag),
    (CopyB, "copyb", Internal, 0, 1, opc_copyb, op_copyb),
}
