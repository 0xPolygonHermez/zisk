//! `ArithEqRow` implementations for every `ArithEq` config air.
//!
//! One `impl_arith_eq_row!` per air (covering the unpacked row and its `…Packed` twin), derived
//! directly from the generated column layout in `zisk_pil` (`pil/src/pil_helpers/traces.rs`). Adding
//! a new alias in `zisk.pil` = one block here. See [`crate::arith_eq_row`] for the design.
//!
//! A config comes in two heights (a plain air and a `Large`), and in two endiannesses: the
//! `big_endian: 1` twins (`*Be`) commit x1..y3 as byte pairs and prove the `*Be` operations. The
//! siblings commit the same columns, but the generated row types are distinct, so each one gets its
//! own block; only the trace alias, the row type names and (for the twins) `big_endian`/`sels` differ.

use crate::impl_arith_eq_row;

// The full air: every column, all 11 operations.
impl_arith_eq_row!(
    unpacked: ArithEqTraceRow,
    packed: ArithEqTraceRowPacked,
    trace: ArithEqTrace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Arith256        => set_sel_arith256,          set_arith256_clk0,
        Arith256Mod     => set_sel_arith256_mod,      set_arith256_mod_clk0,
        Secp256k1Add    => set_sel_secp256k1_add,     set_secp256k1_add_clk0,
        Secp256k1Dbl    => set_sel_secp256k1_dbl,     set_secp256k1_dbl_clk0,
        Bn254CurveAdd   => set_sel_bn254_curve_add,   set_bn254_curve_add_clk0,
        Bn254CurveDbl   => set_sel_bn254_curve_dbl,   set_bn254_curve_dbl_clk0,
        Bn254ComplexAdd => set_sel_bn254_complex_add, set_bn254_complex_add_clk0,
        Bn254ComplexSub => set_sel_bn254_complex_sub, set_bn254_complex_sub_clk0,
        Bn254ComplexMul => set_sel_bn254_complex_mul, set_bn254_complex_mul_clk0,
        Secp256r1Add    => set_sel_secp256r1_add,     set_secp256r1_add_clk0,
        Secp256r1Dbl    => set_sel_secp256r1_dbl,     set_secp256r1_dbl_clk0,
    ]
);

// The full air, tall.
impl_arith_eq_row!(
    unpacked: ArithEqLargeTraceRow,
    packed: ArithEqLargeTraceRowPacked,
    trace: ArithEqLargeTrace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Arith256        => set_sel_arith256,          set_arith256_clk0,
        Arith256Mod     => set_sel_arith256_mod,      set_arith256_mod_clk0,
        Secp256k1Add    => set_sel_secp256k1_add,     set_secp256k1_add_clk0,
        Secp256k1Dbl    => set_sel_secp256k1_dbl,     set_secp256k1_dbl_clk0,
        Bn254CurveAdd   => set_sel_bn254_curve_add,   set_bn254_curve_add_clk0,
        Bn254CurveDbl   => set_sel_bn254_curve_dbl,   set_bn254_curve_dbl_clk0,
        Bn254ComplexAdd => set_sel_bn254_complex_add, set_bn254_complex_add_clk0,
        Bn254ComplexSub => set_sel_bn254_complex_sub, set_bn254_complex_sub_clk0,
        Bn254ComplexMul => set_sel_bn254_complex_mul, set_bn254_complex_mul_clk0,
        Secp256r1Add    => set_sel_secp256r1_add,     set_secp256r1_add_clk0,
        Secp256r1Dbl    => set_sel_secp256r1_dbl,     set_secp256r1_dbl_clk0,
    ]
);

// arith256 + arith256_mod combined.
impl_arith_eq_row!(
    unpacked: Arith256XTraceRow,
    packed: Arith256XTraceRowPacked,
    trace: Arith256XTrace,
    qs: 2,
    use_s: false,
    ceqs: 1,
    opt: [q0, q1, x3_lt, y3_lt],
    sels: [
        Arith256    => set_sel_arith256,     set_arith256_clk0,
        Arith256Mod => set_sel_arith256_mod, set_arith256_mod_clk0,
    ]
);

// arith256 + arith256_mod combined, tall.
impl_arith_eq_row!(
    unpacked: Arith256XLargeTraceRow,
    packed: Arith256XLargeTraceRowPacked,
    trace: Arith256XLargeTrace,
    qs: 2,
    use_s: false,
    ceqs: 1,
    opt: [q0, q1, x3_lt, y3_lt],
    sels: [
        Arith256    => set_sel_arith256,     set_arith256_clk0,
        Arith256Mod => set_sel_arith256_mod, set_arith256_mod_clk0,
    ]
);

// secp256k1 curve add/dbl.
impl_arith_eq_row!(
    unpacked: ArithSecp256K1TraceRow,
    packed: ArithSecp256K1TraceRowPacked,
    trace: ArithSecp256K1Trace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Secp256k1Add => set_sel_secp256k1_add, set_secp256k1_add_clk0,
        Secp256k1Dbl => set_sel_secp256k1_dbl, set_secp256k1_dbl_clk0,
    ]
);

// secp256k1 curve add/dbl, tall.
impl_arith_eq_row!(
    unpacked: ArithSecp256K1LargeTraceRow,
    packed: ArithSecp256K1LargeTraceRowPacked,
    trace: ArithSecp256K1LargeTrace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Secp256k1Add => set_sel_secp256k1_add, set_secp256k1_add_clk0,
        Secp256k1Dbl => set_sel_secp256k1_dbl, set_secp256k1_dbl_clk0,
    ]
);

// bn254: EC curve add/dbl together with complex add/sub/mul (Fp2).
impl_arith_eq_row!(
    unpacked: ArithBn254TraceRow,
    packed: ArithBn254TraceRowPacked,
    trace: ArithBn254Trace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Bn254CurveAdd   => set_sel_bn254_curve_add,   set_bn254_curve_add_clk0,
        Bn254CurveDbl   => set_sel_bn254_curve_dbl,   set_bn254_curve_dbl_clk0,
        Bn254ComplexAdd => set_sel_bn254_complex_add, set_bn254_complex_add_clk0,
        Bn254ComplexSub => set_sel_bn254_complex_sub, set_bn254_complex_sub_clk0,
        Bn254ComplexMul => set_sel_bn254_complex_mul, set_bn254_complex_mul_clk0,
    ]
);

// bn254, tall.
impl_arith_eq_row!(
    unpacked: ArithBn254LargeTraceRow,
    packed: ArithBn254LargeTraceRowPacked,
    trace: ArithBn254LargeTrace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Bn254CurveAdd   => set_sel_bn254_curve_add,   set_bn254_curve_add_clk0,
        Bn254CurveDbl   => set_sel_bn254_curve_dbl,   set_bn254_curve_dbl_clk0,
        Bn254ComplexAdd => set_sel_bn254_complex_add, set_bn254_complex_add_clk0,
        Bn254ComplexSub => set_sel_bn254_complex_sub, set_bn254_complex_sub_clk0,
        Bn254ComplexMul => set_sel_bn254_complex_mul, set_bn254_complex_mul_clk0,
    ]
);

// ---------------------------------------------------------------------------------------------
// Big-endian twins (`big_endian: 1` in zisk.pil): same configs, x1..y3 as `x1_c[2]`... byte pairs,
// and they prove the `*Be` operations (same selector columns).
// ---------------------------------------------------------------------------------------------

// The full air, big-endian.
impl_arith_eq_row!(
    unpacked: ArithEqBeTraceRow,
    packed: ArithEqBeTraceRowPacked,
    trace: ArithEqBeTrace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    big_endian: true,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Arith256Be        => set_sel_arith256,          set_arith256_clk0,
        Arith256ModBe     => set_sel_arith256_mod,      set_arith256_mod_clk0,
        Secp256k1AddBe    => set_sel_secp256k1_add,     set_secp256k1_add_clk0,
        Secp256k1DblBe    => set_sel_secp256k1_dbl,     set_secp256k1_dbl_clk0,
        Bn254CurveAddBe   => set_sel_bn254_curve_add,   set_bn254_curve_add_clk0,
        Bn254CurveDblBe   => set_sel_bn254_curve_dbl,   set_bn254_curve_dbl_clk0,
        Bn254ComplexAddBe => set_sel_bn254_complex_add, set_bn254_complex_add_clk0,
        Bn254ComplexSubBe => set_sel_bn254_complex_sub, set_bn254_complex_sub_clk0,
        Bn254ComplexMulBe => set_sel_bn254_complex_mul, set_bn254_complex_mul_clk0,
        Secp256r1AddBe    => set_sel_secp256r1_add,     set_secp256r1_add_clk0,
        Secp256r1DblBe    => set_sel_secp256r1_dbl,     set_secp256r1_dbl_clk0,
    ]
);

// The full air, big-endian, tall.
impl_arith_eq_row!(
    unpacked: ArithEqBeLargeTraceRow,
    packed: ArithEqBeLargeTraceRowPacked,
    trace: ArithEqBeLargeTrace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    big_endian: true,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Arith256Be        => set_sel_arith256,          set_arith256_clk0,
        Arith256ModBe     => set_sel_arith256_mod,      set_arith256_mod_clk0,
        Secp256k1AddBe    => set_sel_secp256k1_add,     set_secp256k1_add_clk0,
        Secp256k1DblBe    => set_sel_secp256k1_dbl,     set_secp256k1_dbl_clk0,
        Bn254CurveAddBe   => set_sel_bn254_curve_add,   set_bn254_curve_add_clk0,
        Bn254CurveDblBe   => set_sel_bn254_curve_dbl,   set_bn254_curve_dbl_clk0,
        Bn254ComplexAddBe => set_sel_bn254_complex_add, set_bn254_complex_add_clk0,
        Bn254ComplexSubBe => set_sel_bn254_complex_sub, set_bn254_complex_sub_clk0,
        Bn254ComplexMulBe => set_sel_bn254_complex_mul, set_bn254_complex_mul_clk0,
        Secp256r1AddBe    => set_sel_secp256r1_add,     set_secp256r1_add_clk0,
        Secp256r1DblBe    => set_sel_secp256r1_dbl,     set_secp256r1_dbl_clk0,
    ]
);

// arith256 + arith256_mod combined, big-endian.
impl_arith_eq_row!(
    unpacked: Arith256XBeTraceRow,
    packed: Arith256XBeTraceRowPacked,
    trace: Arith256XBeTrace,
    qs: 2,
    use_s: false,
    ceqs: 1,
    big_endian: true,
    opt: [q0, q1, x3_lt, y3_lt],
    sels: [
        Arith256Be    => set_sel_arith256,     set_arith256_clk0,
        Arith256ModBe => set_sel_arith256_mod, set_arith256_mod_clk0,
    ]
);

// arith256 + arith256_mod combined, big-endian, tall.
impl_arith_eq_row!(
    unpacked: Arith256XBeLargeTraceRow,
    packed: Arith256XBeLargeTraceRowPacked,
    trace: Arith256XBeLargeTrace,
    qs: 2,
    use_s: false,
    ceqs: 1,
    big_endian: true,
    opt: [q0, q1, x3_lt, y3_lt],
    sels: [
        Arith256Be    => set_sel_arith256,     set_arith256_clk0,
        Arith256ModBe => set_sel_arith256_mod, set_arith256_mod_clk0,
    ]
);

// secp256k1 curve add/dbl, big-endian.
impl_arith_eq_row!(
    unpacked: ArithSecp256K1BeTraceRow,
    packed: ArithSecp256K1BeTraceRowPacked,
    trace: ArithSecp256K1BeTrace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    big_endian: true,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Secp256k1AddBe => set_sel_secp256k1_add, set_secp256k1_add_clk0,
        Secp256k1DblBe => set_sel_secp256k1_dbl, set_secp256k1_dbl_clk0,
    ]
);

// secp256k1 curve add/dbl, big-endian, tall.
impl_arith_eq_row!(
    unpacked: ArithSecp256K1BeLargeTraceRow,
    packed: ArithSecp256K1BeLargeTraceRowPacked,
    trace: ArithSecp256K1BeLargeTrace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    big_endian: true,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Secp256k1AddBe => set_sel_secp256k1_add, set_secp256k1_add_clk0,
        Secp256k1DblBe => set_sel_secp256k1_dbl, set_secp256k1_dbl_clk0,
    ]
);

// bn254: EC curve add/dbl together with complex add/sub/mul (Fp2), big-endian.
impl_arith_eq_row!(
    unpacked: ArithBn254BeTraceRow,
    packed: ArithBn254BeTraceRowPacked,
    trace: ArithBn254BeTrace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    big_endian: true,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Bn254CurveAddBe   => set_sel_bn254_curve_add,   set_bn254_curve_add_clk0,
        Bn254CurveDblBe   => set_sel_bn254_curve_dbl,   set_bn254_curve_dbl_clk0,
        Bn254ComplexAddBe => set_sel_bn254_complex_add, set_bn254_complex_add_clk0,
        Bn254ComplexSubBe => set_sel_bn254_complex_sub, set_bn254_complex_sub_clk0,
        Bn254ComplexMulBe => set_sel_bn254_complex_mul, set_bn254_complex_mul_clk0,
    ]
);

// bn254, big-endian, tall.
impl_arith_eq_row!(
    unpacked: ArithBn254BeLargeTraceRow,
    packed: ArithBn254BeLargeTraceRowPacked,
    trace: ArithBn254BeLargeTrace,
    qs: 3,
    use_s: true,
    ceqs: 3,
    big_endian: true,
    opt: [q0, q1, q2, s, x3_lt, y3_lt, x_are_different, x_delta_chunk_inv],
    sels: [
        Bn254CurveAddBe   => set_sel_bn254_curve_add,   set_bn254_curve_add_clk0,
        Bn254CurveDblBe   => set_sel_bn254_curve_dbl,   set_bn254_curve_dbl_clk0,
        Bn254ComplexAddBe => set_sel_bn254_complex_add, set_bn254_complex_add_clk0,
        Bn254ComplexSubBe => set_sel_bn254_complex_sub, set_bn254_complex_sub_clk0,
        Bn254ComplexMulBe => set_sel_bn254_complex_mul, set_bn254_complex_mul_clk0,
    ]
);
