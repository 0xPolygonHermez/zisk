//! Opcodes proven by the binary state machines that have **no** `ZiskOp` variant.
//!
//! Every ZisK opcode is already exposed by the `define_ops!` macro as an associated constant on
//! `ZiskOp` (`ZiskOp::ADD`, `ZiskOp::BREV8`, ...), so referencing those directly avoids a second
//! set of names that has to be kept in sync. What is left here are the internal operations that
//! `Arith` assumes and `Binary` proves, which are not part of the ZisK instruction set.

/// `GT`: greater than (signed). Not a RISC-V instruction (`a > b` is compiled as `slt` with the
/// operands swapped), so the transpiler never emits it. `Arith` assumes it on the operation bus to
/// check `0 <= |remainder| < |divisor|` when both are negative, where `|d| < |b|` is exactly
/// `d > b` signed; see `arith_full.rs` and the `assumes_operation` in `arith.pil`.
pub const GT_OP: u8 = 0x08;

/// `GT_W`: the m32 variant of [`GT_OP`]. Nothing emits it -- `Arith` only ever assumes the 64-bit
/// comparisons -- but `Binary` can prove it, since `GT` has a 32-bit variant in the binary table, so
/// this constant records that 0x18 is taken and keeps a future opcode from being assigned to it.
pub const GTW_OP: u8 = GT_OP + M32_OFFSET;

/// `LT_ABS_NP`/`LT_ABS_PN`: absolute value comparisons, |a| < |b| for operands of opposite signs.
/// They occupy the Binary opcode space, so `zisk_ops.rs` must never hand out either code.
pub const LT_ABS_NP_OP: u8 = 0x50;
pub const LT_ABS_PN_OP: u8 = 0x51;

/// `LTU_DIV`: the bus tag for the `LTU` that `Arith` assumes on every division to check
/// `0 <= remainder < divisor` when both are non-negative.
///
/// It is **not** an opcode `Binary` proves, and nothing in the PIL knows about it: the collectors
/// turn it back into [`ZiskOp::LTU`] when they build the input ([`crate::BinaryInput::from`]), so
/// the air only ever sees a plain `LTU` and the operation bus balances exactly as before.
///
/// Its only job is to keep those operations out of FROPS. A frequent operation is counted by the
/// ROM-histogram assembly, which counts *instructions*; an operation `Arith` injects is not one, so
/// the assembly can never see it and the two producers of the multiplicity column would disagree.
/// Giving it a code with no FROPS boxes settles that by construction, and costs one row in the
/// Binary air per injected operation that would otherwise have been frequent.
///
/// The code comes from the range `zisk_ops.rs` reserves for binary (0x59-0x5f); nothing may be
/// assigned to it while this tag uses it.
pub const LTU_DIV_OP: u8 = 0x59;

/// The m32 offset: `Binary` proves `b_op + M32_OFFSET * mode32` (see `binary.pil`).
pub const M32_OFFSET: u8 = 0x10;

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_core::zisk_ops::{OpType, ZiskOp};

    /// The 64-bit opcodes that do have a 32-bit variant, paired with it.
    const M32_PAIRS: [(u8, u8); 12] = [
        (ZiskOp::MINU, ZiskOp::MINU_W),
        (ZiskOp::MIN, ZiskOp::MIN_W),
        (ZiskOp::MAXU, ZiskOp::MAXU_W),
        (ZiskOp::MAX, ZiskOp::MAX_W),
        (ZiskOp::LTU, ZiskOp::LTU_W),
        (ZiskOp::LT, ZiskOp::LT_W),
        (GT_OP, GTW_OP),
        (ZiskOp::EQ, ZiskOp::EQ_W),
        (ZiskOp::ADD, ZiskOp::ADD_W),
        (ZiskOp::SUB, ZiskOp::SUB_W),
        (ZiskOp::LEU, ZiskOp::LEU_W),
        (ZiskOp::LE, ZiskOp::LE_W),
    ];

    /// Opcodes proven by `Binary` that have no `ZiskOp` variant: `GT`/`GT_W` and the absolute
    /// value comparisons, all of them assumed by `Arith`.
    const INTERNAL_OPS: [u8; 4] = [GT_OP, GTW_OP, LT_ABS_NP_OP, LT_ABS_PN_OP];

    /// Every opcode `Binary` can prove, whether or not it is a `ZiskOp`.
    fn binary_opcodes() -> Vec<u8> {
        let mut ops = INTERNAL_OPS.to_vec();
        for code in ZiskOp::MIN_OPCODE..=ZiskOp::MAX_OPCODE {
            if matches!(ZiskOp::try_from_code(code), Ok(op) if op.op_type() == OpType::Binary) {
                ops.push(code);
            }
        }
        ops
    }

    /// The division tag is a bus-level alias, so the only thing that must hold is that its code is
    /// free: not a ZisK opcode and not one of the internal ones.
    #[test]
    fn the_division_tag_uses_a_free_code() {
        assert!(
            ZiskOp::try_from_code(LTU_DIV_OP).is_err(),
            "0x{LTU_DIV_OP:02x} is a ZisK opcode, it cannot also be the division tag"
        );
        assert!(!INTERNAL_OPS.contains(&LTU_DIV_OP));
        for op in binary_opcodes() {
            assert_ne!(op, LTU_DIV_OP, "the division tag collides with a binary opcode");
        }
    }

    #[test]
    fn m32_pairs_are_one_offset_apart() {
        for (op, op_w) in M32_PAIRS {
            assert_eq!(op + M32_OFFSET, op_w, "0x{op:02x} and 0x{op_w:02x} are not an m32 pair");
        }
    }
}
