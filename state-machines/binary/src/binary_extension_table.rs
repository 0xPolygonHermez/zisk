//! The `BinaryExtensionTableSM` module implements the logic for managing the Binary Extension
//! Table.
//!
//! This state machine is responsible for calculating extension binary table rows.

use zisk_core::{P2_11, P2_19, P2_8};

/// Represents operations supported by the Binary Extension Table.
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BinaryExtensionTableOp {
    Sll = 0x31,
    Srl = 0x32,
    Sra = 0x33,
    SllW = 0x34,
    SrlW = 0x35,
    SraW = 0x36,
    SignExtendB = 0x37,
    SignExtendH = 0x38,
    SignExtendW = 0x39,
}

/// The `BinaryExtensionTableSM` struct encapsulates the Binary Extension Table's logic.
pub struct BinaryExtensionTableSM;

impl BinaryExtensionTableSM {
    pub const TABLE_ID: usize = 124;

    /// Calculates the row index in the Binary Extension Table based on the operation and its
    /// inputs.
    ///
    /// # Arguments
    /// * `opcode` - The operation code, as a `BinaryExtensionTableOp`.
    /// * `offset` - The offset value.
    /// * `a` - The first operand.
    /// * `b` - The second operand.
    ///
    /// # Returns
    /// A `u64` representing the calculated row index in the table.
    ///
    /// # Panics
    /// In debug mode, it panics if `offset` > 0x07, `a` > 0xFF, or `b` > 0xFF, as these violate
    /// table constraints.
    pub fn calculate_table_row(opcode: BinaryExtensionTableOp, offset: u64, a: u64, b: u64) -> u64 {
        //lookup_proves(BINARY_EXTENSION_TABLE_ID, [OP, OFFSET, A, B, C0, C1], multiplicity);
        debug_assert!(offset <= 0x07);
        debug_assert!(a <= 0xFF);
        debug_assert!(b <= 0xFF);

        // Calculate the different row offset contributors, according to the PIL
        let offset_a: u64 = a;
        let offset_offset: u64 = offset * P2_8;
        let offset_b: u64 = b * P2_11;
        let offset_opcode: u64 = Self::offset_opcode(opcode);

        offset_a + offset_offset + offset_b + offset_opcode
    }

    /// Computes the opcode offset for a given `BinaryExtensionTableOp`.
    ///
    /// # Arguments
    /// * `opcode` - The operation code as a `BinaryExtensionTableOp`.
    ///
    /// # Returns
    /// A `u64` representing the offset contribution of the opcode.
    fn offset_opcode(opcode: BinaryExtensionTableOp) -> u64 {
        match opcode {
            BinaryExtensionTableOp::Sll => 0,
            BinaryExtensionTableOp::Srl => P2_19,
            BinaryExtensionTableOp::Sra => 2 * P2_19,
            BinaryExtensionTableOp::SllW => 3 * P2_19,
            BinaryExtensionTableOp::SrlW => 4 * P2_19,
            BinaryExtensionTableOp::SraW => 5 * P2_19,
            BinaryExtensionTableOp::SignExtendB => 6 * P2_19,
            BinaryExtensionTableOp::SignExtendH => 6 * P2_19 + P2_11,
            BinaryExtensionTableOp::SignExtendW => 6 * P2_19 + 2 * P2_11,
        }
    }
}
