//! The `BinaryBasicTableSM` module defines the Binary Basic Table State Machine.
//!
//! This state machine is responsible for calculating basic binary table rows.

use zisk_core::{zisk_ops::ZiskOp, P2_16, P2_17, P2_18, P2_19, P2_20, P2_8};

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
    Leu = LEU_OP as u16,
    Le = LE_OP as u16,
    And = AND_OP as u16,
    Or = OR_OP as u16,
    Xor = XOR_OP as u16,
    Sext00 = 0x200,
    SextFF = 0x201,
    Andn = ZiskOp::Andn.code() as u16,
    Orn = ZiskOp::Orn.code() as u16,
    Xnor = ZiskOp::Xnor.code() as u16,
    Brev8 = ZiskOp::Brev8.code() as u16,
    Sh1add = SH1ADD_OP as u16,
    Sh2add = SH2ADD_OP as u16,
    Sh3add = SH3ADD_OP as u16,
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
        debug_assert!(
            cin <= match opcode {
                BinaryBasicTableOp::LtAbsNP => 0x03,
                // The SHxADD carry also holds the bits shifted out of the previous byte
                op if op.shift() != 0 => 1 << op.shift(),
                _ => 0x01,
            }
        );
        debug_assert!(pos_ind <= 0x02);
        debug_assert!(flags <= 0b111_1111);

        // flags = cout + 16*result_is_a + 32*use_first_byte + 64*c_is_signed
        let result_is_a_flag = if (flags & 0b1_0000) != 0 { 1 } else { 0 };

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
            BinaryBasicTableOp::LtAbsPN => P2_20 + 4 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Ltu => P2_20 + P2_19 + 4 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Lt => P2_20 + P2_19 + 5 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Gt => P2_20 + P2_19 + 6 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Eq => P2_20 + P2_19 + 7 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Add => P2_20 + P2_19 + 8 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Sub => P2_20 + P2_19 + 9 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Leu => P2_20 + P2_19 + 10 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Le => P2_20 + P2_19 + 11 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::And => P2_20 + P2_19 + 12 * P2_18 + 4 * P2_17,
            BinaryBasicTableOp::Or => P2_20 + P2_19 + 12 * P2_18 + 5 * P2_17,
            BinaryBasicTableOp::Xor => P2_20 + P2_19 + 12 * P2_18 + 6 * P2_17,
            BinaryBasicTableOp::Sext00 => P2_20 + P2_19 + 12 * P2_18 + 7 * P2_17,
            BinaryBasicTableOp::SextFF => P2_20 + P2_19 + 12 * P2_18 + 8 * P2_17 + P2_16,
            BinaryBasicTableOp::Andn => P2_20 + P2_19 + 12 * P2_18 + 9 * P2_17 + 2 * P2_16,
            BinaryBasicTableOp::Orn => P2_20 + P2_19 + 12 * P2_18 + 10 * P2_17 + 2 * P2_16,
            BinaryBasicTableOp::Xnor => P2_20 + P2_19 + 12 * P2_18 + 11 * P2_17 + 2 * P2_16,
            BinaryBasicTableOp::Brev8 => P2_20 + P2_19 + 12 * P2_18 + 12 * P2_17 + 2 * P2_16,
            // SH1ADD, SH2ADD and SH3ADD take 3, 5 and 9 blocks of P2_17 rows (one per CIN value)
            BinaryBasicTableOp::Sh1add => P2_20 + P2_19 + 12 * P2_18 + 13 * P2_17 + 2 * P2_16,
            BinaryBasicTableOp::Sh2add => P2_20 + P2_19 + 12 * P2_18 + 16 * P2_17 + 2 * P2_16,
            BinaryBasicTableOp::Sh3add => P2_20 + P2_19 + 12 * P2_18 + 21 * P2_17 + 2 * P2_16,
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
            | BinaryBasicTableOp::Xor
            | BinaryBasicTableOp::Andn
            | BinaryBasicTableOp::Orn
            | BinaryBasicTableOp::Xnor
            | BinaryBasicTableOp::Brev8
            | BinaryBasicTableOp::Sh1add
            | BinaryBasicTableOp::Sh2add
            | BinaryBasicTableOp::Sh3add => P2_16,

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
            | BinaryBasicTableOp::Le
            | BinaryBasicTableOp::Sh1add
            | BinaryBasicTableOp::Sh2add
            | BinaryBasicTableOp::Sh3add => P2_17,

            BinaryBasicTableOp::Minu
            | BinaryBasicTableOp::Min
            | BinaryBasicTableOp::Maxu
            | BinaryBasicTableOp::Max
            | BinaryBasicTableOp::Sext00
            | BinaryBasicTableOp::SextFF => P2_16,

            BinaryBasicTableOp::And
            | BinaryBasicTableOp::Or
            | BinaryBasicTableOp::Xor
            | BinaryBasicTableOp::Andn
            | BinaryBasicTableOp::Orn
            | BinaryBasicTableOp::Xnor
            | BinaryBasicTableOp::Brev8 => 0,
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
            | BinaryBasicTableOp::Xor
            | BinaryBasicTableOp::Andn
            | BinaryBasicTableOp::Orn
            | BinaryBasicTableOp::Xnor
            | BinaryBasicTableOp::Brev8
            | BinaryBasicTableOp::Sh1add
            | BinaryBasicTableOp::Sh2add
            | BinaryBasicTableOp::Sh3add => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Size in rows of each region, in the same order as the `OP` fixed column of
    /// `binary_table.pil`. MUST be kept in sync with it.
    const TABLE_LAYOUT: &[(BinaryBasicTableOp, u64)] = &[
        (BinaryBasicTableOp::Minu, P2_18 + P2_17),
        (BinaryBasicTableOp::Min, P2_18 + P2_17),
        (BinaryBasicTableOp::Maxu, P2_18 + P2_17),
        (BinaryBasicTableOp::Max, P2_18 + P2_17),
        (BinaryBasicTableOp::LtAbsNP, P2_20),
        (BinaryBasicTableOp::LtAbsPN, P2_19),
        (BinaryBasicTableOp::Ltu, P2_18),
        (BinaryBasicTableOp::Lt, P2_18),
        (BinaryBasicTableOp::Gt, P2_18),
        (BinaryBasicTableOp::Eq, P2_18),
        (BinaryBasicTableOp::Add, P2_18),
        (BinaryBasicTableOp::Sub, P2_18),
        (BinaryBasicTableOp::Leu, P2_18),
        (BinaryBasicTableOp::Le, P2_18),
        (BinaryBasicTableOp::And, P2_17),
        (BinaryBasicTableOp::Or, P2_17),
        (BinaryBasicTableOp::Xor, P2_17),
        (BinaryBasicTableOp::Sext00, P2_17 + P2_16),
        (BinaryBasicTableOp::SextFF, P2_17 + P2_16),
        (BinaryBasicTableOp::Andn, P2_17),
        (BinaryBasicTableOp::Orn, P2_17),
        (BinaryBasicTableOp::Xnor, P2_17),
        (BinaryBasicTableOp::Brev8, P2_17),
        (BinaryBasicTableOp::Sh1add, 3 * P2_17),
        (BinaryBasicTableOp::Sh2add, 5 * P2_17),
        (BinaryBasicTableOp::Sh3add, 9 * P2_17),
    ];

    /// MUST match `BINARY_TABLE_SIZE` in `binary_table.pil`.
    const BINARY_TABLE_SIZE: u64 = 8_781_824;

    const SH_ADD_OPS: [(BinaryBasicTableOp, u32); 3] = [
        (BinaryBasicTableOp::Sh1add, 1),
        (BinaryBasicTableOp::Sh2add, 2),
        (BinaryBasicTableOp::Sh3add, 3),
    ];

    /// Values exercising the interesting byte patterns: all zeros, all ones, the shifted-out bits
    /// and the addition carry, alone and together.
    const VALUES: [u64; 10] = [
        0,
        1,
        0xFF,
        0x80,
        0xFFFF_FFFF,
        0x8000_0000_0000_0000,
        u64::MAX,
        0x1234_5678_9ABC_DEF0,
        0xE0E0_E0E0_E0E0_E0E0,
        0x0102_0408_1020_4080,
    ];

    #[test]
    fn table_regions_tile_the_whole_table() {
        let mut offset = 0;
        for (op, size) in TABLE_LAYOUT {
            assert_eq!(
                BinaryBasicTableSM::offset_opcode(*op),
                offset,
                "unexpected region offset for {op:?}"
            );
            offset += size;
        }
        assert_eq!(offset, BINARY_TABLE_SIZE, "the regions do not tile BINARY_TABLE_SIZE");
    }

    #[test]
    fn sh_add_rows_stay_inside_their_region() {
        for (op, shift) in SH_ADD_OPS {
            let base = BinaryBasicTableSM::offset_opcode(op);
            let size = ((1 << shift) + 1) * P2_17;

            for cin in 0..=(1u64 << shift) {
                for pos_ind in 0..2 {
                    for a in [0, 1, 0xFF] {
                        for b in [0, 0x80, 0xFF] {
                            let row = BinaryBasicTableSM::calculate_table_row(
                                op, a, b, cin, pos_ind, cin,
                            );

                            // Same decomposition as the fixed columns of binary_table.pil
                            assert_eq!(row, base + a + b * P2_8 + pos_ind * P2_16 + cin * P2_17);
                            assert!(row >= base && row < base + size, "{op:?} row out of region");
                        }
                    }
                }
            }
        }
    }

    /// Mirror of the `OP_SHxADD` case of `binary_table.pil`, for a single byte. Returns `(c, cout)`.
    fn sh_add_table_row(shift: u32, a: u64, b: u64, cin: u64, plast: bool) -> (u64, u64) {
        let sum = ((a << shift) & 0xFF) + b + cin;

        // The sum never overflows twice: ((a << shift) & 0xFF) + b + cin <= 511
        assert!(sum >> 8 <= 1, "byte sum overflowed twice: {sum}");

        (sum & 0xFF, if plast { 0 } else { (sum >> 8) + (a >> (8 - shift)) })
    }

    #[test]
    fn sh_add_byte_chain_matches_the_zisk_op() {
        for (op, shift) in SH_ADD_OPS {
            let zisk_op = match op {
                BinaryBasicTableOp::Sh1add => ZiskOp::Sh1add,
                BinaryBasicTableOp::Sh2add => ZiskOp::Sh2add,
                _ => ZiskOp::Sh3add,
            };

            for a in VALUES {
                for b in VALUES {
                    let (expected, flag) = ZiskOp::execute(zisk_op.code(), a, b);
                    assert!(!flag);

                    let (a_bytes, b_bytes) = (a.to_le_bytes(), b.to_le_bytes());
                    let mut c_bytes = [0u8; 8];
                    let mut cin = 0;

                    for i in 0..8 {
                        let (c, cout) = sh_add_table_row(
                            shift,
                            a_bytes[i] as u64,
                            b_bytes[i] as u64,
                            cin,
                            i == 7,
                        );
                        c_bytes[i] = c as u8;
                        cin = cout;

                        // The carry transports the addition carry plus the shifted out bits
                        assert!(cin <= 1 << shift, "{op:?} carry out of range: {cin}");
                    }

                    // The last carry is discarded, so it never reaches the operation bus
                    assert_eq!(cin, 0);
                    assert_eq!(
                        u64::from_le_bytes(c_bytes),
                        expected,
                        "{op:?} mismatch for a={a:#x} b={b:#x}"
                    );
                }
            }
        }
    }
}
