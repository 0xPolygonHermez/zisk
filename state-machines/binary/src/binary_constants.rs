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

/// `GT_W`: the m32 shadow of [`GT_OP`]. Nothing emits or proves it -- `Arith` only ever assumes the
/// 64-bit comparisons -- so this constant exists purely to record that 0x18 is taken, which is what
/// keeps a future opcode from being assigned to it.
pub const GTW_OP: u8 = GT_OP + M32_OFFSET;

/// `LT_ABS_NP`/`LT_ABS_PN`: absolute value comparisons, |a| < |b| for operands of opposite signs.
/// They occupy the Binary opcode space and reserve their m32 shadows (0x60, 0x61) just the same,
/// so `zisk_ops.rs` must never hand out any of those four codes.
pub const LT_ABS_NP_OP: u8 = 0x50;
pub const LT_ABS_PN_OP: u8 = 0x51;

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

    #[test]
    fn m32_pairs_are_one_offset_apart() {
        for (op, op_w) in M32_PAIRS {
            assert_eq!(op + M32_OFFSET, op_w, "0x{op:02x} and 0x{op_w:02x} are not an m32 pair");
        }
    }

    /// `mode32` is a free witness, so from any `b_op` the Binary air can also prove
    /// `b_op + 0x10`. Every opcode it proves therefore reserves that slot: it must hold either
    /// the operation's own m32 variant or nothing at all. Handing it to an unrelated operation
    /// would let Binary satisfy that operation on the bus with the wrong semantics.
    #[test]
    fn no_binary_opcode_shadows_an_unrelated_operation() {
        for op in binary_opcodes() {
            // Skip the m32 variants themselves, they are the shadow of their 64-bit base.
            if M32_PAIRS.iter().any(|&(_, op_w)| op_w == op) {
                continue;
            }

            let Some(shadow) = op.checked_add(M32_OFFSET) else { continue };

            // The slot is allowed to hold this very operation's m32 variant.
            if M32_PAIRS.contains(&(op, shadow)) {
                continue;
            }

            assert!(
                !INTERNAL_OPS.contains(&shadow),
                "0x{op:02x} shadows the internal binary opcode 0x{shadow:02x}"
            );
            if let Ok(taken) = ZiskOp::try_from_code(shadow) {
                panic!(
                    "0x{op:02x} ({}) shadows 0x{shadow:02x} ({}): the m32 slot of a binary \
                     opcode must stay empty",
                    ZiskOp::try_from_code(op).map(|o| o.name()).unwrap_or("internal"),
                    taken.name(),
                );
            }
        }
    }
}
