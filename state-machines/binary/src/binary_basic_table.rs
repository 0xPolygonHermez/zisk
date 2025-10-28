//! The `BinaryBasicTableSM` module defines the Binary Basic Table State Machine.
//!
//! This state machine is responsible for calculating basic binary table rows.

use zisk_core::{P2_16, P2_17, P2_18, P2_19, P2_8, P2_9};

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
    Ext32 = 0x13,
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
    /// * `last` - The "last" flag.
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
        last: u64,
        flags: u64,
    ) -> u64 {
        debug_assert!(a <= 0xFF);
        debug_assert!(b <= 0xFF);
        debug_assert!(cin <= 0x03);
        debug_assert!(last <= 0x01);
        debug_assert!(flags <= 0x0F);

        // Calculate the different row offset contributors, according to the PIL
        if opcode == BinaryBasicTableOp::Ext32 {
            // Offset calculation for `Ext32` operation.
            let offset_a: u64 = a;
            let offset_cin: u64 = cin * P2_8;
            let offset_result_is_a: u64 = match flags {
                0 => 0,
                2 => P2_9,
                6 => 3 * P2_9,
                _ => {
                    panic!(
                        "BinaryBasicTableSM::calculate_table_row() Unexpected flags for Ext32: {flags}"
                    )
                }
            };
            let offset_opcode: u64 = Self::offset_opcode(opcode);

            offset_a + offset_cin + offset_result_is_a + offset_opcode
        } else {
            // Offset calculation for other operations.
            let offset_a: u64 = a;
            let offset_b: u64 = b * P2_8;
            let offset_last: u64 = if Self::opcode_has_last(opcode) { last * P2_16 } else { 0 };
            let offset_cin: u64 = if Self::opcode_has_cin(opcode) { cin * P2_17 } else { 0 };
            let offset_result_is_a: u64 =
                if Self::opcode_result_is_a(opcode) && ((flags & 0x04) != 0) { P2_18 } else { 0 };
            let offset_opcode: u64 = Self::offset_opcode(opcode);

            offset_a + offset_b + offset_last + offset_cin + offset_result_is_a + offset_opcode
        }
    }

    /// Determines if the given opcode requires a "last" flag.
    fn opcode_has_last(opcode: BinaryBasicTableOp) -> bool {
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
            | BinaryBasicTableOp::Xor => true,
            BinaryBasicTableOp::Ext32 => false,
        }
    }

    /// Determines if the given opcode requires a carry-in value.
    fn opcode_has_cin(opcode: BinaryBasicTableOp) -> bool {
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
            | BinaryBasicTableOp::Le => true,

            BinaryBasicTableOp::And
            | BinaryBasicTableOp::Or
            | BinaryBasicTableOp::Xor
            | BinaryBasicTableOp::Ext32 => false,
        }
    }

    /// Determines if the given opcode's result depends on the "a" operand.
    fn opcode_result_is_a(opcode: BinaryBasicTableOp) -> bool {
        match opcode {
            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max => true,

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
            | BinaryBasicTableOp::Xor
            | BinaryBasicTableOp::Ext32 => false,
        }
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
            BinaryBasicTableOp::Ext32 => 6 * P2_19 + 8 * P2_18 + 3 * P2_17,
        }
    }
}
