//! Classification helpers that decide which binary air can prove a given bus operation.
//!
//! The additions split by operand shape: the packed `BinaryAddHi` airs only cover the ones whose
//! result fits in the low limb, and `BinaryAdd` / `Binary` take the rest. [`add_shape`] is the single
//! source of truth for that split — the counter uses it to bucket operations, the collectors to
//! accept or reject them, and the planner to size the instances — so all three always agree.
//!
//! Operand convention: for `a op b = c`, each of `a`, `b` and `c` is a 64-bit value seen as two
//! 32-bit limbs, `[0]` being the low part and `[1]` the high part.

use zisk_core::zisk_ops::ZiskOp;

/// Number of independent additions packed into one `BinaryAddHi` row.
/// MUST match `adds_x_row` of the `BinaryAddHi` alias in `pil/zisk.pil`.
pub const ADDS_X_ROW: usize = 3;

/// Number of independent additions packed into one `BinaryAddHiLarge` row.
/// MUST match `adds_x_row` of the `BinaryAddHiLarge` alias in `pil/zisk.pil`.
pub const ADDS_X_ROW_LARGE: usize = 5;

/// The widest packing any add-hi air uses, so one row can be built through a fixed-size buffer
/// whatever air is being filled.
pub const MAX_ADDS_X_ROW: usize = ADDS_X_ROW_LARGE;

/// High limb of a negative 32-bit value sign-extended to 64 bits.
pub const NEG_HI: u64 = 0xFFFF_FFFF;

const MASK_32: u64 = 0xFFFF_FFFF;

/// Shape of an `OP_ADD` bus operation, i.e. which add air is able to prove it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddShape {
    /// `a[1] == b[1] == c[1] == 0`: the addition fits entirely in the low limb and does not carry
    /// out of it. `BinaryAddHi` proves it in any of its slots.
    Hi,

    /// `a[1] == c[1] == 0` and `b[1] == 0xFFFF_FFFF`: a two's-complement addition (negative `b`)
    /// whose signed result is non-negative, so the low limb always carries. `BinaryAddHi` proves it
    /// in any of its slots: the carry is what tells the shapes apart, so it doubles as the
    /// `sel_b_hi_is_ff` selector.
    HiNeg,

    /// Any other shape: needs the full 64-bit add (`BinaryAdd`, or `Binary` itself).
    Full,
}

/// Classifies an `OP_ADD` operation by its operand shape, where `a` and `b` are the 64-bit bus
/// operands.
#[inline(always)]
pub fn add_shape(a: u64, b: u64) -> AddShape {
    // The high limb of the result is c[1] = a[1] + b[1] + carry (mod 2^32), where `carry` is the
    // carry out of the low limb.
    let carry = ((a & MASK_32) + (b & MASK_32)) >> 32;

    if (a >> 32) != 0 {
        return AddShape::Full;
    }

    match b >> 32 {
        // c[1] = carry, so c[1] == 0 iff the low limb does not carry.
        0 if carry == 0 => AddShape::Hi,
        // c[1] = 0xFFFF_FFFF + carry (mod 2^32), so c[1] == 0 iff the low limb carries.
        NEG_HI if carry == 1 => AddShape::HiNeg,
        _ => AddShape::Full,
    }
}

/// Determines if the given opcode belongs to the shift family (shifts, rotates and single-bit
/// ops), i.e. the ones whose `b` operand is a shift amount or a bit index.
pub fn opcode_is_shift(opcode: ZiskOp) -> bool {
    match opcode {
        ZiskOp::Sll
        | ZiskOp::Srl
        | ZiskOp::Sra
        | ZiskOp::SllW
        | ZiskOp::SrlW
        | ZiskOp::SraW
        | ZiskOp::Rol
        | ZiskOp::RolW
        | ZiskOp::Ror
        | ZiskOp::RorW
        | ZiskOp::Bclr
        | ZiskOp::Bext
        | ZiskOp::Binv
        | ZiskOp::Bset
        | ZiskOp::SllUW => true,

        ZiskOp::SignExtendB
        | ZiskOp::SignExtendH
        | ZiskOp::SignExtendW
        | ZiskOp::Rev8
        | ZiskOp::OrcB
        | ZiskOp::Cpop
        | ZiskOp::CpopW
        | ZiskOp::Ctz
        | ZiskOp::CtzW
        | ZiskOp::Clz
        | ZiskOp::ClzW
        | ZiskOp::Pack
        | ZiskOp::PackH
        | ZiskOp::PackW => false,

        _ => panic!("opcode_is_shift() got invalid opcode={opcode:?}"),
    }
}

/// Determines if the given opcode is a forward byte-chain operation (ctz family, scanned
/// LSB -> MSB), where each byte's table row is linked to the previous one through the accumulated
/// count in `free_in_c[j][1]`.
pub fn opcode_is_chain(opcode: ZiskOp) -> bool {
    matches!(opcode, ZiskOp::Ctz | ZiskOp::CtzW)
}

/// Determines if the given opcode is a reverse byte-chain operation (clz family, scanned
/// MSB -> LSB).
pub fn opcode_is_chain_rev(opcode: ZiskOp) -> bool {
    matches!(opcode, ZiskOp::Clz | ZiskOp::ClzW)
}

/// Determines if the given opcode is a pack (combine) operation, where the low halves of the two
/// register operands are interleaved into `free_in_a`.
pub fn opcode_is_combine(opcode: ZiskOp) -> bool {
    matches!(opcode, ZiskOp::Pack | ZiskOp::PackH | ZiskOp::PackW)
}

/// Determines if the given opcode represents a shift word (32-bit) operation.
pub fn opcode_is_shift_word(opcode: ZiskOp) -> bool {
    match opcode {
        ZiskOp::SllW | ZiskOp::SrlW | ZiskOp::SraW | ZiskOp::RolW | ZiskOp::RorW => true,

        ZiskOp::Sll
        | ZiskOp::Srl
        | ZiskOp::Sra
        // slli.uw masks the shift amount with 6 bits, like the 64-bit shifts
        | ZiskOp::SllUW
        | ZiskOp::SignExtendB
        | ZiskOp::SignExtendH
        | ZiskOp::SignExtendW
        | ZiskOp::Rev8
        | ZiskOp::OrcB
        | ZiskOp::Rol
        | ZiskOp::Ror
        | ZiskOp::Cpop
        | ZiskOp::CpopW
        | ZiskOp::Ctz
        | ZiskOp::CtzW
        | ZiskOp::Clz
        | ZiskOp::ClzW
        | ZiskOp::Pack
        | ZiskOp::PackH
        | ZiskOp::PackW
        | ZiskOp::Bclr
        | ZiskOp::Bext
        | ZiskOp::Binv
        | ZiskOp::Bset => false,

        _ => panic!("opcode_is_shift_word() got invalid opcode={opcode:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_shape_hi() {
        assert_eq!(add_shape(0, 0), AddShape::Hi);
        assert_eq!(add_shape(1, 2), AddShape::Hi);
        // Largest sum that still fits in the low limb.
        assert_eq!(add_shape(0xFFFF_FFFE, 1), AddShape::Hi);
    }

    #[test]
    fn add_shape_hi_carry_out_of_low_limb_is_not_hi() {
        // a + b == 2^32 => c[1] == 1, so BinaryAddHi cannot prove it.
        assert_eq!(add_shape(0xFFFF_FFFF, 1), AddShape::Full);
    }

    #[test]
    fn add_shape_hi_neg() {
        // b == -1 sign-extended; any a != 0 carries, so the result is non-negative.
        let minus_one = u64::MAX;
        assert_eq!(add_shape(1, minus_one), AddShape::HiNeg);
        assert_eq!(add_shape(0xFFFF_FFFF, minus_one), AddShape::HiNeg);

        // a == 0 does not carry => c[1] == 0xFFFF_FFFF, so it is not a hi shape.
        assert_eq!(add_shape(0, minus_one), AddShape::Full);
    }

    #[test]
    fn add_shape_full_when_a_is_dirty() {
        assert_eq!(add_shape(1 << 32, 0), AddShape::Full);
    }

    #[test]
    fn add_shape_full_when_b_hi_is_neither_zero_nor_all_ones() {
        assert_eq!(add_shape(0, 1 << 32), AddShape::Full);
    }
}
