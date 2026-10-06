//! The `BinaryBasicTableSM` module defines the Binary Basic Table State Machine.
//!
//! This state machine is responsible for calculating basic binary table rows.

use zisk_core::zisk_ops::ZiskOp;

use crate::binary_constants::*;

/// Represents operations supported by the Binary Basic Table.
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u16)]
pub enum BinaryBasicTableOp {
    Minu = ZiskOp::MINU as u16,
    Min = ZiskOp::MIN as u16,
    Maxu = ZiskOp::MAXU as u16,
    Max = ZiskOp::MAX as u16,
    LtAbsNP = LT_ABS_NP_OP as u16,
    LtAbsPN = LT_ABS_PN_OP as u16,
    Ltu = ZiskOp::LTU as u16,
    Lt = ZiskOp::LT as u16,
    Gt = GT_OP as u16,
    Eq = ZiskOp::EQ as u16,
    Add = ZiskOp::ADD as u16,
    Sub = ZiskOp::SUB as u16,
    Leu = ZiskOp::LEU as u16,
    Le = ZiskOp::LE as u16,
    And = ZiskOp::AND as u16,
    Or = ZiskOp::OR as u16,
    Xor = ZiskOp::XOR as u16,
    Andn = ZiskOp::ANDN as u16,
    Orn = ZiskOp::ORN as u16,
    Xnor = ZiskOp::XNOR as u16,
    Brev8 = ZiskOp::BREV8 as u16,
    Sh1add = ZiskOp::SH1ADD as u16,
    Sh2add = ZiskOp::SH2ADD as u16,
    Sh3add = ZiskOp::SH3ADD as u16,
}

impl BinaryBasicTableOp {
    /// Shift amount of the `SHxADD` family, `0` for any other operation.
    ///
    /// The carry of these operations also transports the bits shifted out of the previous byte, so
    /// it ranges in `[0, 2^shift]` instead of being a single bit.
    #[inline(always)]
    pub fn shift(&self) -> u32 {
        match self {
            BinaryBasicTableOp::Sh1add => 1,
            BinaryBasicTableOp::Sh2add => 2,
            BinaryBasicTableOp::Sh3add => 3,
            _ => 0,
        }
    }
}
