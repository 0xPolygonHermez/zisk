//! Unit tests for the big-endian side of the `ArithEq` family: the `*Be` operations, the byte-pair
//! rows of the `big_endian: 1` airs and the operand conversion of their bus data.
//! Declared from `lib.rs` via `#[cfg(test)] #[path = …] mod arith_eq_big_endian_tests;`.

use crate::{
    Arith256Input, ArithEqOp, ArithEqRow, Secp256k1AddInput, ARITH_EQ_LE_OP_NUM, ARITH_EQ_OP_NUM,
};
use proofman_fields::Goldilocks;
use zisk_core::zisk_ops::ZiskOp;
use zisk_pil::{ArithEqBeTraceRow, ArithEqBeTraceRowPacked, ArithEqTraceRow};

/// The op table is the little-endian ops followed by their twins in the same order, and the twins
/// map back and forth by index.
#[test]
fn big_endian_ops_are_the_twins_of_the_little_endian_ones() {
    assert_eq!(ARITH_EQ_OP_NUM, 2 * ARITH_EQ_LE_OP_NUM);
    assert_eq!(ArithEqOp::ALL.len(), ARITH_EQ_OP_NUM);
    for (i, &op) in ArithEqOp::ALL.iter().enumerate() {
        assert_eq!(op.index(), i);
        assert_eq!(op.is_big_endian(), i >= ARITH_EQ_LE_OP_NUM, "{op:?}");
        assert_eq!(op.little_endian(), ArithEqOp::ALL_LE[i % ARITH_EQ_LE_OP_NUM], "{op:?}");
        assert_eq!(op.big_endian(), ArithEqOp::ALL_BE[i % ARITH_EQ_LE_OP_NUM], "{op:?}");
    }
    // The bus opcodes follow: a big-endian opcode is its twin's with bit 7 cleared.
    let pairs = [
        (ZiskOp::ARITH256, ArithEqOp::Arith256Be),
        (ZiskOp::ARITH256_MOD, ArithEqOp::Arith256ModBe),
        (ZiskOp::SECP256K1_ADD, ArithEqOp::Secp256k1AddBe),
        (ZiskOp::SECP256K1_DBL, ArithEqOp::Secp256k1DblBe),
        (ZiskOp::BN254_CURVE_ADD, ArithEqOp::Bn254CurveAddBe),
        (ZiskOp::BN254_CURVE_DBL, ArithEqOp::Bn254CurveDblBe),
        (ZiskOp::BN254_COMPLEX_ADD, ArithEqOp::Bn254ComplexAddBe),
        (ZiskOp::BN254_COMPLEX_SUB, ArithEqOp::Bn254ComplexSubBe),
        (ZiskOp::BN254_COMPLEX_MUL, ArithEqOp::Bn254ComplexMulBe),
        (ZiskOp::SECP256R1_ADD, ArithEqOp::Secp256r1AddBe),
        (ZiskOp::SECP256R1_DBL, ArithEqOp::Secp256r1DblBe),
    ];
    for (le_code, be_op) in pairs {
        assert_eq!(ArithEqOp::from_opcode(le_code), Some(be_op.little_endian()));
        assert_eq!(ArithEqOp::from_opcode(le_code & 0x7F), Some(be_op));
    }
}

/// A big-endian row commits each 16-bit chunk as `[low byte, high byte]`, which is what the PIL
/// recombines as `x1 = x1_c[0] + 256 * x1_c[1]`; a little-endian row keeps the chunk.
#[test]
fn big_endian_rows_split_the_chunk_into_its_bytes() {
    let mut row = ArithEqBeTraceRow::<Goldilocks>::default();
    <ArithEqBeTraceRow<Goldilocks> as ArithEqRow<Goldilocks>>::set_x1(&mut row, 0xABCD);
    <ArithEqBeTraceRow<Goldilocks> as ArithEqRow<Goldilocks>>::set_y3(&mut row, 0x0102);
    assert_eq!(row.get_all_x1_c(), [0xCD, 0xAB]);
    assert_eq!(row.get_all_y3_c(), [0x02, 0x01]);
    const { assert!(<ArithEqBeTraceRow<Goldilocks> as ArithEqRow<Goldilocks>>::BIG_ENDIAN) };

    let mut packed = ArithEqBeTraceRowPacked::<Goldilocks>::default();
    <ArithEqBeTraceRowPacked<Goldilocks> as ArithEqRow<Goldilocks>>::set_x2(&mut packed, 0xFF01);
    assert_eq!(packed.get_all_x2_c(), [0x01, 0xFF]);

    let mut le = ArithEqTraceRow::<Goldilocks>::default();
    <ArithEqTraceRow<Goldilocks> as ArithEqRow<Goldilocks>>::set_x1(&mut le, 0xABCD);
    assert_eq!(le.get_x1(), 0xABCD);
    const { assert!(!<ArithEqTraceRow<Goldilocks> as ArithEqRow<Goldilocks>>::BIG_ENDIAN) };
}

/// A big-endian op drives the same selector column as its twin, so the fill can index the
/// selectors by the little-endian op.
#[test]
fn big_endian_ops_drive_the_little_endian_selector_columns() {
    let mut row = ArithEqBeTraceRow::<Goldilocks>::default();
    <ArithEqBeTraceRow<Goldilocks> as ArithEqRow<Goldilocks>>::set_sel(
        &mut row,
        ArithEqOp::Bn254ComplexMulBe,
        true,
    );
    assert!(row.get_sel_bn254_complex_mul());
    // The little-endian variant is not an op of a big-endian air: no column, no-op.
    <ArithEqBeTraceRow<Goldilocks> as ArithEqRow<Goldilocks>>::set_sel(
        &mut row,
        ArithEqOp::Bn254ComplexMul,
        false,
    );
    assert!(row.get_sel_bn254_complex_mul(), "a little-endian op is a no-op on a big-endian row");
}

/// The bus data of a big-endian op carries the operands as they sit in memory: the 256-bit integer
/// 0x0102...1f20 stored big-endian, read as four little-endian words. `from_bus` turns them into the
/// little-endian limbs of that integer; the addresses are untouched.
#[test]
fn from_bus_converts_big_endian_operands_to_limbs() {
    let bytes: [u8; 32] = core::array::from_fn(|i| i as u8 + 1);
    let mem_words: [u64; 4] =
        core::array::from_fn(|i| u64::from_le_bytes(bytes[8 * i..8 * i + 8].try_into().unwrap()));
    let limbs: [u64; 4] =
        [0x191A1B1C1D1E1F20, 0x1112131415161718, 0x090A0B0C0D0E0F10, 0x0102030405060708];

    // arith256: [op, op_type, a, b, step, 5 addresses, a[4], b[4], c[4]]
    let mut data = [0u64; 22];
    data[3] = 0x9000;
    data[4] = 77;
    data[5..10].copy_from_slice(&[1, 2, 3, 4, 5]);
    data[10..14].copy_from_slice(&mem_words);
    data[14..18].copy_from_slice(&mem_words);
    data[18..22].copy_from_slice(&[0, 0, 0, 1]);
    let be = Arith256Input::from_bus(&data, true);
    assert_eq!(be.a, limbs);
    assert_eq!(be.b, limbs);
    assert_eq!(be.c, [1u64.swap_bytes(), 0, 0, 0], "a big-endian 1 is the top word's top byte");
    assert_eq!((be.addr, be.step, be.a_addr, be.dh_addr), (0x9000, 77, 1, 5));
    let le = Arith256Input::from_bus(&data, false);
    assert_eq!(le.a, mem_words, "a little-endian op takes the words as they are");

    // secp256k1_add: [op, op_type, a, b, step, 2 addresses, p1[8], p2[8]]: two coordinates each.
    let mut data = [0u64; 23];
    data[7..11].copy_from_slice(&mem_words);
    data[11..15].copy_from_slice(&[0, 0, 0, 1]);
    data[15..23].copy_from_slice(&[mem_words, mem_words].concat());
    let be = Secp256k1AddInput::from_bus(&data, true);
    assert_eq!(be.p1[..4], limbs);
    assert_eq!(be.p1[4..], [1u64.swap_bytes(), 0, 0, 0]);
    assert_eq!(be.p2[..4], limbs);
    assert_eq!(be.p2[4..], limbs);
}
