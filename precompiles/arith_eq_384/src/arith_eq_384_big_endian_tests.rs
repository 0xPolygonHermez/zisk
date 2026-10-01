//! Unit tests for the big-endian side of the `ArithEq384` family.

use crate::{
    arith_eq_384_op_is_big_endian, Arith384ModInput, ArithEq384Row, Bls12_381CurveAddInput,
    ARITH_EQ_384_U64S,
};
use proofman_fields::Goldilocks;
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{ArithEq384BeTraceRow, ArithEq384BeTraceRowPacked, ArithEq384TraceRow};

#[test]
fn opcodes_map_to_their_endianness() {
    for (le, be) in [
        (ZiskOp::ARITH384_MOD, ZiskOp::ARITH384_MOD_BE),
        (ZiskOp::BLS12_381_CURVE_ADD, ZiskOp::BLS12_381_CURVE_ADD_BE),
        (ZiskOp::BLS12_381_CURVE_DBL, ZiskOp::BLS12_381_CURVE_DBL_BE),
        (ZiskOp::BLS12_381_COMPLEX_ADD, ZiskOp::BLS12_381_COMPLEX_ADD_BE),
        (ZiskOp::BLS12_381_COMPLEX_SUB, ZiskOp::BLS12_381_COMPLEX_SUB_BE),
        (ZiskOp::BLS12_381_COMPLEX_MUL, ZiskOp::BLS12_381_COMPLEX_MUL_BE),
    ] {
        assert_eq!(arith_eq_384_op_is_big_endian(le), Some(false));
        assert_eq!(arith_eq_384_op_is_big_endian(be), Some(true));
        assert_eq!(be, le & 0x7F, "a big-endian opcode is its twin's with bit 7 cleared");
    }
    assert_eq!(arith_eq_384_op_is_big_endian(ZiskOp::SECP256K1_ADD), None);
}

#[test]
fn big_endian_rows_split_the_chunk_into_its_bytes() {
    let mut row = ArithEq384BeTraceRow::<Goldilocks>::default();
    <ArithEq384BeTraceRow<Goldilocks> as ArithEq384Row<Goldilocks>>::set_x1(&mut row, 0xABCD);
    assert_eq!(row.get_all_x1_c(), [0xCD, 0xAB]);
    const { assert!(<ArithEq384BeTraceRow<Goldilocks> as ArithEq384Row<Goldilocks>>::BIG_ENDIAN) };

    let mut packed = ArithEq384BeTraceRowPacked::<Goldilocks>::default();
    <ArithEq384BeTraceRowPacked<Goldilocks> as ArithEq384Row<Goldilocks>>::set_y2(
        &mut packed,
        0xFF01,
    );
    assert_eq!(packed.get_all_y2_c(), [0x01, 0xFF]);

    let mut le = ArithEq384TraceRow::<Goldilocks>::default();
    <ArithEq384TraceRow<Goldilocks> as ArithEq384Row<Goldilocks>>::set_x1(&mut le, 0xABCD);
    assert_eq!(le.get_x1(), 0xABCD);
    const { assert!(!<ArithEq384TraceRow<Goldilocks> as ArithEq384Row<Goldilocks>>::BIG_ENDIAN) };
}

/// A 384-bit big-endian operand (bytes 1..48 from the most significant) read as six little-endian
/// words becomes the little-endian limbs of that integer; a point is two such operands.
#[test]
fn from_bus_converts_big_endian_operands_to_limbs() {
    let bytes: [u8; 48] = core::array::from_fn(|i| i as u8 + 1);
    let mem_words: [u64; ARITH_EQ_384_U64S] =
        core::array::from_fn(|i| u64::from_le_bytes(bytes[8 * i..8 * i + 8].try_into().unwrap()));
    let limbs: [u64; ARITH_EQ_384_U64S] = core::array::from_fn(|i| {
        u64::from_be_bytes(bytes[48 - 8 * (i + 1)..48 - 8 * i].try_into().unwrap())
    });
    assert_eq!(limbs[0], 0x292A2B2C2D2E2F30);
    assert_eq!(limbs[5], 0x0102030405060708);

    // arith384_mod: [op, op_type, a, b, step, 5 addresses, a[6], b[6], c[6], module[6]]
    let mut data = [0u64; 34];
    data[10..16].copy_from_slice(&mem_words);
    data[28..34].copy_from_slice(&[0, 0, 0, 0, 0, 1]);
    let be = Arith384ModInput::from_bus(&data, true);
    assert_eq!(be.a, limbs);
    assert_eq!(be.module, [1u64.swap_bytes(), 0, 0, 0, 0, 0]);
    assert_eq!(Arith384ModInput::from_bus(&data, false).a, mem_words);

    // bls12_381_curve_add: [op, op_type, a, b, step, 2 addresses, p1[12], p2[12]]
    let mut data = [0u64; 31];
    data[7..13].copy_from_slice(&mem_words);
    data[13..19].copy_from_slice(&[0, 0, 0, 0, 0, 1]);
    let be = Bls12_381CurveAddInput::from_bus(&data, true);
    assert_eq!(be.p1[..6], limbs);
    assert_eq!(be.p1[6..], [1u64.swap_bytes(), 0, 0, 0, 0, 0]);
}
