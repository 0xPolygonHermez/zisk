//! The `BinaryBasicTableSM` module defines the Binary Basic Table State Machine.
//!
//! This state machine is responsible for calculating basic binary table rows.

use zisk_core::{P2_16, P2_17, P2_18, P2_19, P2_8};

use crate::binary_constants::*;

/// Represents operations supported by the Binary Basic Table.
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u16)]
pub enum BinaryBasicTableOp {
    Minu = MINU_OP as u16,
    Min = MIN_OP as u16,
    Maxu = MAXU_OP as u16,
    Max = MAX_OP as u16,
    LtAbsNP = LT_ABS_NP_OP as u16,
    LtAbsPN = LT_ABS_PN_OP as u16,
    Ltu = LTU_OP as u16,
    Lt = LT_OP as u16,
    Gt = GT_OP as u16,
    Eq = EQ_OP as u16,
    Add = ADD_OP as u16,
    Sub = SUB_OP as u16,
    Leu = LEUW_OP as u16,
    Le = LE_OP as u16,
    And = AND_OP as u16,
    Or = OR_OP as u16,
    Xor = XOR_OP as u16,
    Sext00 = 0x200,
    SextFF = 0x201,
}

/// The `BinaryBasicTableSM` struct represents the Binary Basic Table State Machine.
pub struct BinaryBasicTableSM;

impl BinaryBasicTableSM {
    pub const TABLE_ID: usize = 125;

    /// Calculates the table row offset based on the provided parameters.
    ///
    /// # Arguments
    /// * `opcode` - The operation code (`BinaryBasicTableOp`).
    /// * `a` - The first operand a.
    /// * `b` - The second operand b.
    /// * `cin` - The carry-in value.
    /// * `pos_ind` - The position indicator.
    /// * `flags` - The flags value.
    ///
    /// # Returns
    /// The calculated table row offset.
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_table_row(
        opcode: BinaryBasicTableOp,
        a: u64,
        b: u64,
        cin: u64,
        pos_ind: u64,
        flags: u64,
    ) -> u64 {
        debug_assert!(a <= 0xFF);
        debug_assert!(b <= 0xFF);
        debug_assert!(cin <= 0x01);
        debug_assert!(pos_ind <= 0x02);
        debug_assert!(flags <= 0b1111);

        // flags = cout + 2*result_is_a + 4*use_first_byte + 8*c_is_signed
        let result_is_a_flag = if (flags & 0b10) != 0 { 1 } else { 0 };

        // Calculate the different row offset contributors
        let offset_opcode: u64 = Self::offset_opcode(opcode);
        let offset_a: u64 = a;
        let offset_b: u64 = b * P2_8;
        let offset_pos_ind: u64 = pos_ind * Self::offset_pos_ind(opcode);
        let offset_cin: u64 = cin * Self::offset_cin(opcode);
        let offset_result_is_a: u64 = result_is_a_flag * Self::offset_result_is_a(opcode, pos_ind);

        offset_opcode + offset_a + offset_b + offset_pos_ind + offset_cin + offset_result_is_a
    }

    /// Computes the opcode offset for the given operation.
    fn offset_opcode(opcode: BinaryBasicTableOp) -> u64 {
        match opcode {
            BinaryBasicTableOp::Minu => 0,
            BinaryBasicTableOp::Min => P2_18 + P2_17,
            BinaryBasicTableOp::Maxu => 2 * P2_18 + 2 * P2_17,
            BinaryBasicTableOp::Max => 3 * P2_18 + 3 * P2_17,
            BinaryBasicTableOp::LtAbsNP => 4 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::LtAbsPN => P2_19 + 4 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Ltu => 2 * P2_19 + 4 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Lt => 2 * P2_19 + 5 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Gt => 2 * P2_19 + 6 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Eq => 2 * P2_19 + 7 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Add => 2 * P2_19 + 8 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Sub => 2 * P2_19 + 9 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Leu => 2 * P2_19 + 10 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Le => 2 * P2_19 + 11 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::And => 2 * P2_19 + 12 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Or => 2 * P2_19 + 12 * P2_18 + 5 * P2_17,
            BinaryBasicTableOp::Xor => 2 * P2_19 + 12 * P2_18 + 6 * P2_17,
            BinaryBasicTableOp::Sext00 => 2 * P2_19 + 12 * P2_18 + 7 * P2_17,
            BinaryBasicTableOp::SextFF => 2 * P2_19 + 12 * P2_18 + 8 * P2_17 + P2_16,
        }
    }

    /// Computes the position indicator offset for the given operation.
    fn offset_pos_ind(opcode: BinaryBasicTableOp) -> u64 {
        match opcode {
            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max => P2_18,

            BinaryBasicTableOp::LtAbsNP
            | BinaryBasicTableOp::LtAbsPN
            | BinaryBasicTableOp::Ltu
            | BinaryBasicTableOp::Lt
            | BinaryBasicTableOp::Gt
            | BinaryBasicTableOp::Eq
            | BinaryBasicTableOp::Add
            | BinaryBasicTableOp::Sub
            | BinaryBasicTableOp::Leu
            | BinaryBasicTableOp::Le
            | BinaryBasicTableOp::And
            | BinaryBasicTableOp::Or
            | BinaryBasicTableOp::Xor => P2_16,

            BinaryBasicTableOp::Sext00 | BinaryBasicTableOp::SextFF => 0,
        }
    }

    /// Computes the carry-in offset for the given operation.
    fn offset_cin(opcode: BinaryBasicTableOp) -> u64 {
        match opcode {
            BinaryBasicTableOp::LtAbsNP | BinaryBasicTableOp::LtAbsPN => P2_18,

            BinaryBasicTableOp::Ltu
            | BinaryBasicTableOp::Lt
            | BinaryBasicTableOp::Gt
            | BinaryBasicTableOp::Eq
            | BinaryBasicTableOp::Add
            | BinaryBasicTableOp::Sub
            | BinaryBasicTableOp::Leu
            | BinaryBasicTableOp::Le => P2_17,

            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max
            | BinaryBasicTableOp::Sext00
            | BinaryBasicTableOp::SextFF => P2_16,

            BinaryBasicTableOp::And | BinaryBasicTableOp::Or | BinaryBasicTableOp::Xor => 0,
        }
    }

    /// Computes the result_is_a offset for the given operation.
    fn offset_result_is_a(opcode: BinaryBasicTableOp, pos_ind: u64) -> u64 {
        match opcode {
            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max => (1 - pos_ind) * P2_17,

            BinaryBasicTableOp::Sext00 | BinaryBasicTableOp::SextFF => P2_17,

            BinaryBasicTableOp::LtAbsNP
            | BinaryBasicTableOp::LtAbsPN
            | BinaryBasicTableOp::Ltu
            | BinaryBasicTableOp::Lt
            | BinaryBasicTableOp::Gt
            | BinaryBasicTableOp::Eq
            | BinaryBasicTableOp::Add
            | BinaryBasicTableOp::Sub
            | BinaryBasicTableOp::Leu
            | BinaryBasicTableOp::Le
            | BinaryBasicTableOp::And
            | BinaryBasicTableOp::Or
            | BinaryBasicTableOp::Xor => 0,
        }
    }
}
