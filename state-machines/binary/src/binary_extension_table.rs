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
    SllUw = ZiskOp::SllUW.code(),
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
            // slli.uw is shift-family too (6-bit B range), placed after the single-bit blocks.
            BinaryExtensionTableOp::SllUw => 18 * P2_17 + 10 * P2_11,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Size in rows of each region, in the same order as the `OP` fixed column of
    /// `binary_extension_table.pil`. MUST be kept in sync with it.
    const TABLE_LAYOUT: &[(BinaryExtensionTableOp, u64)] = &[
        (BinaryExtensionTableOp::Sll, P2_17),
        (BinaryExtensionTableOp::Srl, P2_17),
        (BinaryExtensionTableOp::Sra, P2_17),
        (BinaryExtensionTableOp::SllW, P2_17),
        (BinaryExtensionTableOp::SrlW, P2_17),
        (BinaryExtensionTableOp::SraW, P2_17),
        (BinaryExtensionTableOp::SextB, P2_11),
        (BinaryExtensionTableOp::SextH, P2_11),
        (BinaryExtensionTableOp::SextW, P2_11),
        (BinaryExtensionTableOp::Rev8, P2_11),
        (BinaryExtensionTableOp::OrcB, P2_11),
        (BinaryExtensionTableOp::Rol, P2_17),
        (BinaryExtensionTableOp::RolW, P2_17),
        (BinaryExtensionTableOp::Ror, P2_17),
        (BinaryExtensionTableOp::RorW, P2_17),
        (BinaryExtensionTableOp::Cpop, P2_11),
        (BinaryExtensionTableOp::CpopW, P2_11),
        (BinaryExtensionTableOp::Ctz, P2_17),
        (BinaryExtensionTableOp::CtzW, P2_17),
        (BinaryExtensionTableOp::Clz, P2_17),
        (BinaryExtensionTableOp::ClzW, P2_17),
        (BinaryExtensionTableOp::Pack, P2_11),
        (BinaryExtensionTableOp::PackH, P2_11),
        (BinaryExtensionTableOp::PackW, P2_11),
        (BinaryExtensionTableOp::Bclr, P2_17),
        (BinaryExtensionTableOp::Bext, P2_17),
        (BinaryExtensionTableOp::Binv, P2_17),
        (BinaryExtensionTableOp::Bset, P2_17),
        (BinaryExtensionTableOp::SllUw, P2_17),
    ];

    /// MUST match `BINARY_EXTENSION_TABLE_SIZE` in `binary_extension_table.pil`.
    const BINARY_EXTENSION_TABLE_SIZE: u64 = 2_510_848;

    #[test]
    fn table_regions_tile_the_whole_table() {
        let mut offset = 0;
        for (op, size) in TABLE_LAYOUT {
            assert_eq!(
                BinaryExtensionTableSM::offset_opcode(*op),
                offset,
                "unexpected region offset for {op:?}"
            );
            offset += size;
        }
        assert_eq!(
            offset, BINARY_EXTENSION_TABLE_SIZE,
            "the regions do not tile BINARY_EXTENSION_TABLE_SIZE"
        );
    }

    #[test]
    fn sll_uw_rows_stay_inside_their_region() {
        let base = BinaryExtensionTableSM::offset_opcode(BinaryExtensionTableOp::SllUw);

        for b in [0, 1, 31, 63] {
            for offset in 0..8 {
                for a in [0, 1, 0xFF] {
                    let row = BinaryExtensionTableSM::calculate_table_row(
                        BinaryExtensionTableOp::SllUw,
                        offset,
                        a,
                        b,
                    );

                    // Same decomposition as the fixed columns of binary_extension_table.pil
                    assert_eq!(row, base + a + offset * P2_8 + b * P2_11);
                    assert!(row >= base && row < base + P2_17);
                }
            }
        }
    }

    /// Mirror of the `OP_SLL_UW` case of `binary_extension_table.pil`, for a single byte.
    fn sll_uw_table_row(offset: u32, a: u64, b: u64) -> u64 {
        if offset >= 4 {
            return 0;
        }
        // The 64-bit result drops whatever crosses bit 63 (the PIL masks it with MASK_64)
        let bits_to_shift = (b & 0x3F) + 8 * offset as u64;
        if bits_to_shift < 64 {
            a << bits_to_shift
        } else {
            0
        }
    }

    #[test]
    fn sll_uw_byte_chain_matches_the_zisk_op() {
        let values = [
            0u64,
            1,
            0xFF,
            0x8000_0000,
            0xFFFF_FFFF,
            0x1234_5678_9ABC_DEF0,
            u64::MAX,
            0x0102_0408_1020_4080,
        ];

        for a in values {
            for b in 0..64u64 {
                // The instruction sets m32, so the bus (and hence the witness) only ever carries
                // the low half of a: that masking is the zero extension the operation needs.
                let bus_a = a & 0xFFFF_FFFF;
                let (expected, flag) = ZiskOp::execute(ZiskOp::SllUW.code(), bus_a, b);
                assert!(!flag);

                let a_bytes = bus_a.to_le_bytes();
                let mut out: u64 = 0;
                for (offset, byte) in a_bytes.iter().enumerate() {
                    out += sll_uw_table_row(offset as u32, *byte as u64, b);
                }

                assert_eq!(out, expected, "mismatch for a={bus_a:#x} b={b}");
            }
        }
    }
}
