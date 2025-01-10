//! This module defines constants for binary operation opcodes in both 32-bit and 64-bit variants.
//!
//! These constants are derived from the `ZiskOp` enum and represent the numeric opcodes for each
//! operation.

use zisk_core::zisk_ops::ZiskOp;

/// Binary 32 bits opcodes
pub const MINUW_OP: u8 = ZiskOp::MinuW.code();
pub const MINW_OP: u8 = ZiskOp::MinW.code();
pub const MAXUW_OP: u8 = ZiskOp::MaxuW.code();
pub const MAXW_OP: u8 = ZiskOp::MaxW.code();
pub const LTUW_OP: u8 = ZiskOp::LtuW.code();
pub const LTW_OP: u8 = ZiskOp::LtW.code();
pub const EQW_OP: u8 = ZiskOp::EqW.code();
pub const ADDW_OP: u8 = ZiskOp::AddW.code();
pub const SUBW_OP: u8 = ZiskOp::SubW.code();
pub const LEUW_OP: u8 = ZiskOp::LeuW.code();
pub const LEW_OP: u8 = ZiskOp::LeW.code();

/// Binary 64 bits opcodes
pub const MINU_OP: u8 = ZiskOp::Minu.code();
pub const MIN_OP: u8 = ZiskOp::Min.code();
pub const MAXU_OP: u8 = ZiskOp::Maxu.code();
pub const MAX_OP: u8 = ZiskOp::Max.code();
pub const LT_ABS_NP_OP: u8 = 0x06;
pub const LT_ABS_PN_OP: u8 = 0x07;
pub const LTU_OP: u8 = ZiskOp::Ltu.code();
pub const LT_OP: u8 = ZiskOp::Lt.code();
pub const GT_OP: u8 = 0x0a;
pub const EQ_OP: u8 = ZiskOp::Eq.code();
pub const ADD_OP: u8 = ZiskOp::Add.code();
pub const SUB_OP: u8 = ZiskOp::Sub.code();
pub const LEU_OP: u8 = ZiskOp::Leu.code();
pub const LE_OP: u8 = ZiskOp::Le.code();
pub const AND_OP: u8 = ZiskOp::And.code();
pub const OR_OP: u8 = ZiskOp::Or.code();
pub const XOR_OP: u8 = ZiskOp::Xor.code();
