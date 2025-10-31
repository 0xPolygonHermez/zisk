//! The `BinaryBasicTableSM` module defines the Binary Basic Table State Machine.
//!
//! This state machine is responsible for calculating basic binary table rows.

use zisk_core::{P2_16, P2_17, P2_18, P2_19, P2_8};

use crate::binary_constants::*;

/// Represents operations supported by the Binary Basic Table.
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BinaryBasicTableOp {
    Minu = MINU_OP,
    Min = MIN_OP,
    Maxu = MAXU_OP,
    Max = MAX_OP,
    LtAbsNP = LT_ABS_NP_OP,
    LtAbsPN = LT_ABS_PN_OP,
    Ltu = LTU_OP,
    Lt = LT_OP,
    Gt = GT_OP,
    Eq = EQ_OP,
    Add = ADD_OP,
    Sub = SUB_OP,
    Leu = LEUW_OP,
    Le = LE_OP,
    And = AND_OP,
    Or = OR_OP,
    Xor = XOR_OP,
    Sext00 = 0x13,
    SextFF = 0x14,
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
        let offset_result_is_a: u64 = result_is_a_flag * Self::offset_result_is_a(opcode);

        offset_opcode + offset_a + offset_b + offset_pos_ind + offset_cin + offset_result_is_a
    }

    /// Computes the opcode offset for the given operation.
    fn offset_opcode(opcode: BinaryBasicTableOp) -> u64 {
        match opcode {
            BinaryBasicTableOp::Minu => 0,
            BinaryBasicTableOp::Min => P2_19,
            BinaryBasicTableOp::Maxu => 2 * P2_19,
            BinaryBasicTableOp::Max => 3 * P2_19,
            BinaryBasicTableOp::LtAbsNP => 4 * P2_19,
            BinaryBasicTableOp::LtAbsPN => 5 * P2_19,
            BinaryBasicTableOp::Ltu => 6 * P2_19,
            BinaryBasicTableOp::Lt => 6 * P2_19 + P2_18,
            BinaryBasicTableOp::Gt => 6 * P2_19 + 2 * P2_18,
            BinaryBasicTableOp::Eq => 6 * P2_19 + 3 * P2_18,
            BinaryBasicTableOp::Add => 6 * P2_19 + 4 * P2_18,
            BinaryBasicTableOp::Sub => 6 * P2_19 + 5 * P2_18,
            BinaryBasicTableOp::Leu => 6 * P2_19 + 6 * P2_18,
            BinaryBasicTableOp::Le => 6 * P2_19 + 7 * P2_18,
            BinaryBasicTableOp::And => 6 * P2_19 + 8 * P2_18,
            BinaryBasicTableOp::Or => 6 * P2_19 + 8 * P2_18 + P2_17,
            BinaryBasicTableOp::Xor => 6 * P2_19 + 8 * P2_18 + 2 * P2_17,
            BinaryBasicTableOp::Sext00 => 6 * P2_19 + 8 * P2_18 + 3 * P2_17,
            BinaryBasicTableOp::SextFF => 6 * P2_19 + 9 * P2_18 + 3 * P2_17,
        }
    }

    /// Computes the position indicator offset for the given operation.
    fn offset_pos_ind(opcode: BinaryBasicTableOp) -> u64 {
        match opcode {
            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max
            | BinaryBasicTableOp::LtAbsNP
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

            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max
            | BinaryBasicTableOp::Ltu
            | BinaryBasicTableOp::Lt
            | BinaryBasicTableOp::Gt
            | BinaryBasicTableOp::Eq
            | BinaryBasicTableOp::Add
            | BinaryBasicTableOp::Sub
            | BinaryBasicTableOp::Leu
            | BinaryBasicTableOp::Le => P2_17,

            BinaryBasicTableOp::Sext00 | BinaryBasicTableOp::SextFF => P2_16,

            BinaryBasicTableOp::And | BinaryBasicTableOp::Or | BinaryBasicTableOp::Xor => 0,
        }
    }

    /// Computes the result_is_a offset for the given operation.
    fn offset_result_is_a(opcode: BinaryBasicTableOp) -> u64 {
        match opcode {
            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max
            | BinaryBasicTableOp::Sext00
            | BinaryBasicTableOp::SextFF => P2_18,

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
