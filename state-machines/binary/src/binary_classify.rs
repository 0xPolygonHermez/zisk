//! Classification helpers that decide which binary air can prove a given bus operation.
//!
//! Two of the binary airs come in pairs where the cheaper one only covers a subset of the operand
//! shapes: `BinaryAddHi` vs `BinaryAdd` for additions, and `BinaryExtension` vs
//! `BinaryExtensionFull` for the extension ops. These predicates are the single source of truth
//! for that split — the counter uses them to bucket operations, the collectors to accept or reject
//! them, and the planner to size the instances — so all three always agree.
//!
//! Operand convention: for `a op b = c`, each of `a`, `b` and `c` is a 64-bit value seen as two
//! 32-bit limbs, `[0]` being the low part and `[1]` the high part.

use zisk_common::CollectSkipper;
use zisk_core::zisk_ops::ZiskOp;

/// Number of independent additions packed into one `BinaryAddHi` row.
/// MUST match `adds_x_row` in `state-machines/binary/pil/binary_add_hi.pil`.
pub const ADDS_X_ROW: usize = 3;

/// High limb of a negative 32-bit value sign-extended to 64 bits.
pub const NEG_HI: u64 = 0xFFFF_FFFF;

/// Largest shift amount / bit index representable by the reduced extension air's `free_in_b`.
const LS_6_BITS: u64 = 0x3F;

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

/// Which add shapes a dedicated air is responsible for.
///
/// Additions can be proven by more than one air — `Binary` and `BinaryAdd` take any shape, while the
/// packed `BinaryAddHi` only takes the low-limb ones — so the planner decides per shape where they go
/// and every collector filters by what it was given. Exactly one instance must accept a given
/// operation, otherwise it would be proven twice or not at all.
///
/// This is the filter of the dedicated airs, which take a prefix of a shape; the general `Binary` air
/// takes the tail and filters with [`ShapeDrop`] instead.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct AddScope {
    /// Take the additions that need the full 64-bit add ([`AddShape::Full`]).
    pub full: bool,

    /// Take the additions whose result fits in the low limb ([`AddShape::Hi`] and
    /// [`AddShape::HiNeg`]).
    pub hi: bool,
}

impl AddScope {
    /// Takes no additions at all.
    pub const NONE: Self = Self { full: false, hi: false };

    /// Takes every addition, whatever its shape.
    pub const ALL: Self = Self { full: true, hi: true };

    /// Returns `true` when an addition with these operands belongs to this scope.
    #[inline(always)]
    pub fn accepts(&self, a: u64, b: u64) -> bool {
        match add_shape(a, b) {
            AddShape::Full => self.full,
            AddShape::Hi | AddShape::HiNeg => self.hi,
        }
    }

    /// Returns `true` when no addition at all belongs to this scope.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        !self.full && !self.hi
    }
}

/// How many additions of one shape an instance has to let pass before taking them.
///
/// A dedicated add air takes a *prefix* of its shape's operations — as many as fill whole instances
/// — and the general `Binary` air takes the tail in the room its instances have already paid for.
/// Its collectors therefore need to drop that prefix, which is what this describes.
///
/// The distinction between [`ShapeDrop::all`] and [`ShapeDrop::first`] matters for the frequent
/// operations: while dropping, frops of the shape are dropped too, so they stay accounted for by the
/// dedicated air. `all` keeps dropping to the end of the chunk, which is what makes the dedicated air
/// the sole owner of that chunk's frops for the shape; `first(n)` hands the ones past the boundary
/// over to `Binary`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeDrop {
    /// `true` when every operation of the shape belongs to another air.
    all: bool,

    /// Operations of the shape to let pass before taking them. Unused when `all`.
    skipper: CollectSkipper,
}

impl ShapeDrop {
    /// Every operation of the shape belongs to another air.
    pub fn all() -> Self {
        Self { all: true, skipper: CollectSkipper::new(0) }
    }

    /// Every operation of the shape is ours.
    pub fn none() -> Self {
        Self { all: false, skipper: CollectSkipper::new(0) }
    }

    /// The first `n` non-frop operations of the shape belong to another air; the rest are ours.
    pub fn first(n: u64) -> Self {
        Self { all: false, skipper: CollectSkipper::new(n) }
    }

    /// Returns `true` when the current operation of the shape is ours to take.
    ///
    /// `is_frop` marks frequent operations, which never take a row: they do not count towards the
    /// boundary, but they are dropped along with the prefix so the dedicated air accounts for them.
    #[inline(always)]
    pub fn accepts(&mut self, is_frop: bool) -> bool {
        if self.all {
            return false;
        }
        !self.skipper.should_skip_query(!is_frop)
    }
}

/// What one instance of an add-capable air (`Binary` or `BinaryAdd`) collects from one chunk.
///
/// Besides the usual count and skip over the stream it accepts, it carries which additions belong to
/// another air: those are dropped, so the room an instance has already paid for can take the residual
/// of a shape instead of forcing a whole extra instance of a dedicated air.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BinaryCollectInfo {
    /// Operations to collect from the accepted stream.
    pub count: u64,

    /// Operations of the accepted stream to skip before collecting.
    pub skipper: CollectSkipper,

    /// Low-limb additions that belong to a dedicated air.
    pub hi_drop: ShapeDrop,

    /// Additions needing the full 64-bit add that belong to a dedicated air.
    pub full_drop: ShapeDrop,

    /// Whether the chunk must be walked to its end because it still holds frops to account for.
    pub force_execute_to_end: bool,
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
        | ZiskOp::Bset => true,

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

/// Returns `true` when a `BinaryE` operation can only be proven by `BinaryExtensionFull`.
///
/// The reduced air drops the columns that the full one uses to carry the "dirty" parts of the
/// operands (`free_in_b_bit6`, `free_in_b_bit7` and `b[2]`) plus the byte-chain selectors, so it
/// only covers operations whose unused operand parts are zero:
///
/// * byte-chain families (`CTZ`, `CTZ_W`, `CLZ`, `CLZ_W`) always need the full air, since the
///   reduced one has no `op_is_chain` / `op_is_chain_rev` columns at all;
/// * shift family: the reduced air carries the whole amount in `free_in_b`, so `b` must fit in
///   6 bits (which also forces `b[1] == 0`);
/// * combine (pack) family: the reduced air forces both high limbs to zero;
/// * the remaining single-source ops: the reduced air forces the bus `a` operand to zero, which
///   is what the transpiler emits for them (`src_a` is the immediate 0).
#[inline(always)]
pub fn extension_requires_full(op: u8, a: u64, b: u64) -> bool {
    let opcode = ZiskOp::try_from_code(op).expect("extension_requires_full(): invalid ZiskOp code");

    if opcode_is_chain(opcode) || opcode_is_chain_rev(opcode) {
        return true;
    }

    if opcode_is_shift(opcode) {
        return b > LS_6_BITS;
    }

    if opcode_is_combine(opcode) {
        return (a >> 32) != 0 || (b >> 32) != 0;
    }

    a != 0
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

    /// `ShapeDrop::all` never yields an operation, so the dedicated air stays the sole owner of the
    /// chunk — frops included.
    #[test]
    fn shape_drop_all_never_accepts() {
        let mut drop = ShapeDrop::all();
        for is_frop in [false, true, false, false] {
            assert!(!drop.accepts(is_frop));
        }
    }

    /// `ShapeDrop::none` yields everything.
    #[test]
    fn shape_drop_none_accepts_everything() {
        let mut drop = ShapeDrop::none();
        for is_frop in [false, true, false] {
            assert!(drop.accepts(is_frop));
        }
    }

    /// `ShapeDrop::first(n)` lets the first `n` real operations pass to the dedicated air and takes
    /// the rest. Frops in the dropped prefix go with it; the ones past the boundary are ours.
    #[test]
    fn shape_drop_first_hands_over_the_tail() {
        let mut drop = ShapeDrop::first(2);

        // A frop inside the prefix does not advance the boundary, and is dropped.
        assert!(!drop.accepts(true));
        // The two operations of the prefix.
        assert!(!drop.accepts(false));
        assert!(!drop.accepts(false));
        // From here on everything is ours, frops included.
        assert!(drop.accepts(true));
        assert!(drop.accepts(false));
        assert!(drop.accepts(false));
    }

    /// `first(0)` is the same as taking everything.
    #[test]
    fn shape_drop_first_zero_takes_everything() {
        let mut drop = ShapeDrop::first(0);
        assert!(drop.accepts(false));
        assert!(drop.accepts(true));
    }

    #[test]
    fn extension_chain_always_requires_full() {
        for op in [ZiskOp::Ctz, ZiskOp::CtzW, ZiskOp::Clz, ZiskOp::ClzW] {
            assert!(extension_requires_full(op.code(), 0, 0));
        }
    }

    #[test]
    fn extension_shift_requires_full_only_when_amount_is_dirty() {
        let sll = ZiskOp::Sll.code();
        assert!(!extension_requires_full(sll, 0x1234, 63));
        assert!(extension_requires_full(sll, 0x1234, 64));
        assert!(extension_requires_full(sll, 0x1234, 1 << 32));
    }

    #[test]
    fn extension_combine_requires_full_only_when_a_high_limb_is_dirty() {
        let pack = ZiskOp::Pack.code();
        assert!(!extension_requires_full(pack, 0xFFFF_FFFF, 0xFFFF_FFFF));
        assert!(extension_requires_full(pack, 1 << 32, 0));
        assert!(extension_requires_full(pack, 0, 1 << 32));
    }

    #[test]
    fn extension_single_source_requires_full_only_when_bus_a_is_set() {
        let rev8 = ZiskOp::Rev8.code();
        assert!(!extension_requires_full(rev8, 0, u64::MAX));
        assert!(extension_requires_full(rev8, 1, u64::MAX));
    }
}
