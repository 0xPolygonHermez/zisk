//! Classification helpers that decide which binary air can prove a given bus operation.
//!
//! The additions split by operand shape: the packed `BinaryAddHi` airs only cover the ones whose
//! result fits in the low limb, and `BinaryAdd` / `Binary` take the rest. [`add_shape`] is the single
//! source of truth for that split — the counter uses it to bucket operations, the collectors to
//! accept or reject them, and the planner to size the instances — so all three always agree.
//!
//! Operand convention: for `a op b = c`, each of `a`, `b` and `c` is a 64-bit value seen as two
//! 32-bit limbs, `[0]` being the low part and `[1]` the high part.

use crate::{KIND_ADD_FULL, KIND_ADD_HI, KIND_BASIC, KIND_SH3ADD_ADD, KIND_SH3ADD_HI};
use zisk_core::zisk_ops::ZiskOp;

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

/// Shape of an `OP_SH3ADD` bus operation, i.e. which air is able to prove it.
///
/// `SH3ADD` computes `c = b + (a << 3)`. Both packed airs fold the shift into the addition they
/// already prove — the bits `a << 3` carries out of a limb and the addition carry land on the same
/// place — but each does it under its own limitation, so an operation belongs to the first shape
/// whose air can take it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sh3addShape {
    /// `BinaryAddHi` proves it: the whole 64-bit result is zero above the low limb, exactly the
    /// condition its `AddShape` slots are built for. `a` must be a clean 32-bit value, since the
    /// air pins the high half of `a` on the bus to zero.
    Hi,

    /// `BinaryAdd` proves it: `a` is a clean non-negative 32-bit value and the low limb carries at
    /// most once, which is what its boolean carry can hold.
    Add,

    /// Neither packed air can: only `Binary` proves it, byte by byte.
    Full,
}

/// Classifies an `OP_SH3ADD` operation by its operands, where `a` and `b` are the 64-bit bus
/// operands and `a` is the one that gets shifted.
///
/// The verdict is a function of the operands alone, so the counter, the collectors and the planner
/// all reach it without sharing state — the same contract [`add_shape`] has.
#[inline(always)]
pub fn sh3add_shape(a: u64, b: u64) -> Sh3addShape {
    // Both packed airs pin the high half of `a` on the bus to zero, so a shifted operand that does
    // not fit in 32 bits — or a negative one, whose sign extension fills the high half — is out of
    // reach for either of them.
    if (a >> 32) != 0 {
        return Sh3addShape::Full;
    }

    // `a << 3` needs 35 bits, so the sum with the low limb of `b` needs 36: it cannot overflow u64,
    // and its carry out of the low limb is what tells the shapes apart, exactly as in `add_shape`.
    let sum = ((a & MASK_32) << 3) + (b & MASK_32);
    let carry = sum >> 32;

    match b >> 32 {
        // BinaryAddHi shape 1: b is a clean 32-bit value and the low limb does not carry, so the
        // whole result fits in it and the high halves of a, b and c are all zero.
        0 if carry == 0 => Sh3addShape::Hi,

        // BinaryAddHi shape 2: b is a sign-extended negative value, so the high limb only wraps to
        // zero when the low one carries exactly once.
        NEG_HI if carry == 1 => Sh3addShape::Hi,

        // BinaryAdd takes any b, but its carry out of the low limb is a bit: it needs
        // 8*a[0] + b[0] < 2^33. The high limb never sees the widened term (a[1] is zero), so its
        // own carry is a bit for free and puts no further condition.
        _ if carry <= 1 => Sh3addShape::Add,

        _ => Sh3addShape::Full,
    }
}

/// The kind of the add family a `Binary` bus operation belongs to.
///
/// This is the ONE place the split is decided. The counter buckets operations with it to size the
/// instances, and every collector buckets them with it again to decide what to take — so a plan and
/// the collection that fulfils it can never disagree about where an operation belongs, which is what
/// would otherwise show up as a bus that does not balance.
///
/// `op` must be a `Binary`-type opcode; the caller has already filtered by operation type.
#[inline(always)]
pub fn add_family_kind(op: u8, a: u64, b: u64) -> usize {
    if op == ZiskOp::Add.code() {
        match add_shape(a, b) {
            AddShape::Hi | AddShape::HiNeg => KIND_ADD_HI,
            AddShape::Full => KIND_ADD_FULL,
        }
    } else if op == ZiskOp::Sh3add.code() {
        match sh3add_shape(a, b) {
            Sh3addShape::Hi => KIND_SH3ADD_HI,
            Sh3addShape::Add => KIND_SH3ADD_ADD,
            // Neither packed air can fold the shift into its addition, so only `Binary` proves it:
            // to the planner it is one more basic operation.
            Sh3addShape::Full => KIND_BASIC,
        }
    } else {
        KIND_BASIC
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

    /// The classification must agree with what each air's constraints actually admit, which is the
    /// only thing that keeps the counter, the collectors and the planner from disagreeing.
    ///
    /// `BinaryAddHi` materializes only the low limb and pins the high halves of a and c to zero, so
    /// it proves exactly the operations whose 64-bit result has a zero high limb. `BinaryAdd` keeps
    /// a boolean carry, so it needs 8*a[0] + b[0] < 2^33. Both need a clean 32-bit a.
    fn sh3add_reference(a: u64, b: u64) -> Sh3addShape {
        let c = b.wrapping_add(a.wrapping_shl(3));
        let a_is_clean = (a >> 32) == 0;
        let hi_provable = a_is_clean && (c >> 32) == 0 && matches!(b >> 32, 0 | NEG_HI);
        let add_provable = a_is_clean && ((a & MASK_32) << 3) + (b & MASK_32) < (1u64 << 33);

        if hi_provable {
            Sh3addShape::Hi
        } else if add_provable {
            Sh3addShape::Add
        } else {
            Sh3addShape::Full
        }
    }

    #[test]
    fn sh3add_shape_matches_what_the_airs_prove() {
        let interesting = [
            0u64,
            1,
            7,
            8,
            0x1FFF_FFFF, // largest a whose a<<3 still fits in 32 bits
            0x2000_0000,
            0xFFFF_FFFF,
            0x1_0000_0000, // dirty: does not fit in 32 bits
            u64::MAX,      // dirty: negative sign-extended
            0xFFFF_FFFF_0000_0008,
            0xA000_0000, // a zisk RAM base address
            0xFFFF_FFFE,
        ];
        for &a in &interesting {
            for &b in &interesting {
                assert_eq!(sh3add_shape(a, b), sh3add_reference(a, b), "a=0x{a:X} b=0x{b:X}");
            }
        }
    }

    #[test]
    fn sh3add_shape_matches_on_random_operands() {
        // Deterministic xorshift: no dev-dependency, and a failure is always reproducible.
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..200_000 {
            // Mostly clean 32-bit operands, which is what address arithmetic produces, plus some
            // dirty ones so the Full branch is exercised too.
            let r = next();
            let a = if r & 3 == 0 { r } else { r & MASK_32 };
            let b = if r & 12 == 0 { next() } else { next() & MASK_32 };
            assert_eq!(sh3add_shape(a, b), sh3add_reference(a, b), "a=0x{a:X} b=0x{b:X}");
        }
    }

    #[test]
    fn sh3add_address_arithmetic_is_the_cheap_shape() {
        // base + index*8 with a 32-bit base and a small index: the common case must land on the
        // packed air, since that is the whole point of teaching it SH3ADD.
        let base = 0xA000_1000u64;
        for index in [0u64, 1, 2, 100, 1000, 0x10_0000] {
            assert_eq!(sh3add_shape(index, base), Sh3addShape::Hi, "index={index}");
        }
    }

    #[test]
    fn sh3add_a_must_be_a_clean_32_bit_value() {
        assert_eq!(sh3add_shape(1 << 32, 0), Sh3addShape::Full);
        assert_eq!(sh3add_shape(u64::MAX, 0), Sh3addShape::Full, "negative a is dirty too");
    }

    /// The kind an operation is given must be one the airs that prove it can actually take. This is
    /// the contract between `add_family_kind` and the `proves` arrays in `binary_kinds`: the planner
    /// routes by kind, so a kind an air cannot prove would be routed to it and fail in the witness.
    #[test]
    fn every_kind_is_provable_by_the_airs_that_claim_it() {
        use crate::{add_family, KIND_BASIC};
        use zisk_core::zisk_ops::ZiskOp;

        let interesting = [
            0u64,
            1,
            8,
            0x1FFF_FFFF,
            0x2000_0000,
            0xA000_1000,
            0xFFFF_FFFF,
            0x1_0000_0000,
            u64::MAX,
            0xFFFF_FFFF_0000_0008,
        ];

        // Every air, granted one instance so `add_family` reports what each proves.
        let airs = add_family([1; crate::ADD_AIRS]);

        for &op in &[ZiskOp::Add.code(), ZiskOp::Sh3add.code()] {
            for &a in &interesting {
                for &b in &interesting {
                    let kind = add_family_kind(op, a, b);

                    // Some air must prove it, or the operation has nowhere to go at all.
                    assert!(
                        airs.iter().any(|air| air.proves[kind]),
                        "op={op:#x} a={a:#X} b={b:#X} lands on kind {kind}, which no air proves",
                    );

                    // A basic kind means neither packed family can take it, and the reverse must
                    // hold too: a non-basic kind must be provable by a packed air.
                    if kind != KIND_BASIC {
                        assert!(
                            airs.iter().filter(|air| !air.proves[KIND_BASIC]).any(|a| a.proves[kind]),
                            "op={op:#x} a={a:#X} b={b:#X} is kind {kind} but no packed air proves it",
                        );
                    }
                }
            }
        }
    }

    /// A `Binary`-type opcode that is neither of the two split ones is a basic operation, whatever
    /// its operands: nothing else may sneak into a packed air.
    #[test]
    fn other_opcodes_are_always_basic() {
        use zisk_core::zisk_ops::ZiskOp;
        for op in
            [ZiskOp::And, ZiskOp::Or, ZiskOp::Xor, ZiskOp::Sub, ZiskOp::Sh1add, ZiskOp::Sh2add]
        {
            assert_eq!(add_family_kind(op.code(), 1, 2), crate::KIND_BASIC, "{op:?}");
        }
    }

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
