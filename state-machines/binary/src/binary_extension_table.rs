//! The `BinaryExtensionTableSM` module implements the logic for managing the Binary Extension
//! Table.
//!
//! This state machine is responsible for calculating extension binary table rows.

use zisk_core::{zisk_ops::ZiskOp, P2_11, P2_17, P2_8};

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
    Ctz = ZiskOp::Ctz.code(),
    CtzW = ZiskOp::CtzW.code(),
    Clz = ZiskOp::Clz.code(),
    ClzW = ZiskOp::ClzW.code(),
    Pack = ZiskOp::Pack.code(),
    PackH = ZiskOp::PackH.code(),
    PackW = ZiskOp::PackW.code(),
    Bclr = ZiskOp::Bclr.code(),
    Bext = ZiskOp::Bext.code(),
    Binv = ZiskOp::Binv.code(),
    Bset = ZiskOp::Bset.code(),
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
    /// In debug mode, it panics if `offset` > 0x07, `a` > 0xFF, or `b` > 0x3F, as these violate
    /// table constraints. Only the low 6 bits of B are relevant (the shift amount is masked with
    /// LS_6_BITS / LS_5_BITS), so B is enumerated over 0..63 and each B-using block is 2^17 rows.
    pub fn calculate_table_row(opcode: BinaryExtensionTableOp, offset: u64, a: u64, b: u64) -> u64 {
        //lookup_proves(BINARY_EXTENSION_TABLE_ID, [OP, OFFSET, A, B, C0, C1], multiplicity);
        debug_assert!(offset <= 0x07);
        debug_assert!(a <= 0xFF);
        debug_assert!(b <= 0x3F);

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
        // Every B-using block (shift / rotate / single-bit families) is 2^17 rows (2^8 A *
        // 2^3 offset * 2^6 B); the single-input blocks (sext / rev8 / orcb / cpop / pack) are
        // 2^11 rows; the byte-chain blocks (ctz / clz) are 2^17 rows (2^6 acc_in * 2^3 offset *
        // 2^8 A). Offsets accumulate in the same order the OP column is laid out in the PIL.
        match opcode {
            BinaryExtensionTableOp::Sll => 0,
            BinaryExtensionTableOp::Srl => P2_17,
            BinaryExtensionTableOp::Sra => 2 * P2_17,
            BinaryExtensionTableOp::SllW => 3 * P2_17,
            BinaryExtensionTableOp::SrlW => 4 * P2_17,
            BinaryExtensionTableOp::SraW => 5 * P2_17,
            BinaryExtensionTableOp::SextB => 6 * P2_17,
            BinaryExtensionTableOp::SextH => 6 * P2_17 + P2_11,
            BinaryExtensionTableOp::SextW => 6 * P2_17 + 2 * P2_11,
            BinaryExtensionTableOp::Rev8 => 6 * P2_17 + 3 * P2_11,
            BinaryExtensionTableOp::OrcB => 6 * P2_17 + 4 * P2_11,
            BinaryExtensionTableOp::Rol => 6 * P2_17 + 5 * P2_11,
            BinaryExtensionTableOp::RolW => 7 * P2_17 + 5 * P2_11,
            BinaryExtensionTableOp::Ror => 8 * P2_17 + 5 * P2_11,
            BinaryExtensionTableOp::RorW => 9 * P2_17 + 5 * P2_11,
            BinaryExtensionTableOp::Cpop => 10 * P2_17 + 5 * P2_11,
            BinaryExtensionTableOp::CpopW => 10 * P2_17 + 6 * P2_11,
            // Chain ops: the fourth `calculate_table_row` argument carries acc_in (not B),
            // contributing acc_in * P2_11 as the outer dimension within the block.
            BinaryExtensionTableOp::Ctz => 10 * P2_17 + 7 * P2_11,
            BinaryExtensionTableOp::CtzW => 11 * P2_17 + 7 * P2_11,
            BinaryExtensionTableOp::Clz => 12 * P2_17 + 7 * P2_11,
            BinaryExtensionTableOp::ClzW => 13 * P2_17 + 7 * P2_11,
            // Pack ops are single-block (B unused), placed after the four chain blocks.
            BinaryExtensionTableOp::Pack => 14 * P2_17 + 7 * P2_11,
            BinaryExtensionTableOp::PackH => 14 * P2_17 + 8 * P2_11,
            BinaryExtensionTableOp::PackW => 14 * P2_17 + 9 * P2_11,
            // Single-bit ops are shift-family (6-bit B range), placed after the pack blocks.
            BinaryExtensionTableOp::Bclr => 14 * P2_17 + 10 * P2_11,
            BinaryExtensionTableOp::Bext => 15 * P2_17 + 10 * P2_11,
            BinaryExtensionTableOp::Binv => 16 * P2_17 + 10 * P2_11,
            BinaryExtensionTableOp::Bset => 17 * P2_17 + 10 * P2_11,
        }
    }
}
