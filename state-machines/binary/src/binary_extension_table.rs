//! The `BinaryExtensionTableSM` module implements the logic for managing the Binary Extension
//! Table.
//!
//! This state machine is responsible for calculating extension binary table rows.

use zisk_core::{zisk_ops::ZiskOp, P2_11, P2_19, P2_8};

/// Represents operations supported by the Binary Extension Table.
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BinaryExtensionTableOp {
    Sll = ZiskOp::Sll.code(),
    Srl = ZiskOp::Srl.code(),
    Sra = ZiskOp::Sra.code(),
    SllW = ZiskOp::SllW.code(),
    SrlW = ZiskOp::SrlW.code(),
    SraW = ZiskOp::SraW.code(),
    SextB = ZiskOp::SignExtendB.code(),
    SextH = ZiskOp::SignExtendH.code(),
    SextW = ZiskOp::SignExtendW.code(),
    Rev8 = ZiskOp::Rev8.code(),
    OrcB = ZiskOp::OrcB.code(),
    Rol = ZiskOp::Rol.code(),
    RolW = ZiskOp::RolW.code(),
    Ror = ZiskOp::Ror.code(),
    RorW = ZiskOp::RorW.code(),
    Cpop = ZiskOp::Cpop.code(),
    CpopW = ZiskOp::CpopW.code(),
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
            BinaryExtensionTableOp::SextB => 6 * P2_19,
            BinaryExtensionTableOp::SextH => 6 * P2_19 + P2_11,
            BinaryExtensionTableOp::SextW => 6 * P2_19 + 2 * P2_11,
            BinaryExtensionTableOp::Rev8 => 6 * P2_19 + 3 * P2_11,
            BinaryExtensionTableOp::OrcB => 6 * P2_19 + 4 * P2_11,
            BinaryExtensionTableOp::Rol => 6 * P2_19 + 5 * P2_11,
            BinaryExtensionTableOp::RolW => 7 * P2_19 + 5 * P2_11,
            BinaryExtensionTableOp::Ror => 8 * P2_19 + 5 * P2_11,
            BinaryExtensionTableOp::RorW => 9 * P2_19 + 5 * P2_11,
            BinaryExtensionTableOp::Cpop => 10 * P2_19 + 5 * P2_11,
            BinaryExtensionTableOp::CpopW => 10 * P2_19 + 6 * P2_11,
        }
    }
}
