/* Elliptic-curve and quadratic-extension precompiles.
 *
 * Points are affine, (x, y); complex numbers (re, im), over F_p[i], i^2 = -1.
 * Each coordinate is n words: 4 for a 256-bit field, 6 for BLS12-381's.
 * An addition or a product takes its operands at a and b and writes the
 * result over a; a doubling works in place at b
 * (core/src/zisk_ops.rs; zisk_precomp_helpers).
 *
 * The additions are the chord rule, which needs x1 != x2: adding a point to
 * itself or to its negation is a division by zero, as in the emulator.
 * Nothing checks that a point is on its curve; the formulas are what the
 * operation computes, on any operands.
 */

/* Short Weierstrass y^2 = x^3 + A x + B: the chord through P1 and P2. */
val weierstrass_add : (int, int, int, int, int) -> (int, int)
function weierstrass_add(p, x1, y1, x2, y2) = {
  let s = fdiv(y2 - y1, x2 - x1, p);
  let x3 = (s * s - x1 - x2) % p;
  (x3, (s * (x1 - x3) - y1) % p)
}

/* and the tangent at P1, for the curve's A. */
val weierstrass_dbl : (int, int, int, int) -> (int, int)
function weierstrass_dbl(p, a, x1, y1) = {
  let s = fdiv(3 * x1 * x1 + a, 2 * y1, p);
  let x3 = (s * s - 2 * x1) % p;
  (x3, (s * (x1 - x3) - y1) % p)
}

/* P1 at a plus P2 at b, over a. */
val curve_add : (bits(64), bits(64), int, int, string) -> unit
function curve_add(a, b, p, n, what) = {
  check_aligned(a, what);
  check_aligned(b, what);
  let (x1, y1) = read_pair(a, n, p);
  let (x2, y2) = read_pair(b, n, p);
  write_pair(a, n, weierstrass_add(p, x1, y1, x2, y2))
}

/* 2 P1 at b, in place, on a curve with coefficient A = coef. */
val curve_dbl : (bits(64), int, int, int, string) -> unit
function curve_dbl(b, p, coef, n, what) = {
  check_aligned(b, what);
  let (x1, y1) = read_pair(b, n, p);
  write_pair(b, n, weierstrass_dbl(p, coef, x1, y1))
}

enum complex_op = {CADD, CSUB, CMUL}

/* f1 at a op f2 at b, over a. */
val complex : (bits(64), bits(64), int, int, complex_op, string) -> unit
function complex(a, b, p, n, op, what) = {
  check_aligned(a, what);
  check_aligned(b, what);
  let (x1, y1) = read_pair(a, n, p);
  let (x2, y2) = read_pair(b, n, p);
  write_pair(a, n, match op {
    CADD => ((x1 + x2) % p, (y1 + y2) % p),
    CSUB => ((x1 - x2) % p, (y1 - y2) % p),
    CMUL => ((x1 * x2 - y1 * y2) % p, (y1 * x2 + x1 * y2) % p)
  })
}

/* secp256k1: y^2 = x^3 + 7. */
union clause zisk_op = ZOP_SECP256K1_ADD : unit
function clause zisk_op_of_code (0xf4) = Some(ZOP_SECP256K1_ADD())
function clause zisk_exec_mem (ZOP_SECP256K1_ADD(), a, b, _) = {
  curve_add(a, b, SECP256K1_P, 4, "secp256k1_add");
  Some((zeros(), false))
}

union clause zisk_op = ZOP_SECP256K1_DBL : unit
function clause zisk_op_of_code (0xf5) = Some(ZOP_SECP256K1_DBL())
function clause zisk_exec_mem (ZOP_SECP256K1_DBL(), _, b, _) = {
  curve_dbl(b, SECP256K1_P, 0, 4, "secp256k1_dbl");
  Some((zeros(), false))
}

/* secp256r1 (P-256): y^2 = x^3 - 3x + B. */
union clause zisk_op = ZOP_SECP256R1_ADD : unit
function clause zisk_op_of_code (0xe8) = Some(ZOP_SECP256R1_ADD())
function clause zisk_exec_mem (ZOP_SECP256R1_ADD(), a, b, _) = {
  curve_add(a, b, SECP256R1_P, 4, "secp256r1_add");
  Some((zeros(), false))
}

union clause zisk_op = ZOP_SECP256R1_DBL : unit
function clause zisk_op_of_code (0xe9) = Some(ZOP_SECP256R1_DBL())
function clause zisk_exec_mem (ZOP_SECP256R1_DBL(), _, b, _) = {
  curve_dbl(b, SECP256R1_P, -3, 4, "secp256r1_dbl");
  Some((zeros(), false))
}

/* BN254's G1: y^2 = x^3 + 3; and F_p^2. */
union clause zisk_op = ZOP_BN254_CURVE_ADD : unit
function clause zisk_op_of_code (0xfa) = Some(ZOP_BN254_CURVE_ADD())
function clause zisk_exec_mem (ZOP_BN254_CURVE_ADD(), a, b, _) = {
  curve_add(a, b, BN254_P, 4, "bn254_curve_add");
  Some((zeros(), false))
}

union clause zisk_op = ZOP_BN254_CURVE_DBL : unit
function clause zisk_op_of_code (0xfb) = Some(ZOP_BN254_CURVE_DBL())
function clause zisk_exec_mem (ZOP_BN254_CURVE_DBL(), _, b, _) = {
  curve_dbl(b, BN254_P, 0, 4, "bn254_curve_dbl");
  Some((zeros(), false))
}

union clause zisk_op = ZOP_BN254_COMPLEX_ADD : unit
function clause zisk_op_of_code (0xfc) = Some(ZOP_BN254_COMPLEX_ADD())
function clause zisk_exec_mem (ZOP_BN254_COMPLEX_ADD(), a, b, _) = {
  complex(a, b, BN254_P, 4, CADD, "bn254_complex_add");
  Some((zeros(), false))
}

union clause zisk_op = ZOP_BN254_COMPLEX_SUB : unit
function clause zisk_op_of_code (0xfd) = Some(ZOP_BN254_COMPLEX_SUB())
function clause zisk_exec_mem (ZOP_BN254_COMPLEX_SUB(), a, b, _) = {
  complex(a, b, BN254_P, 4, CSUB, "bn254_complex_sub");
  Some((zeros(), false))
}

union clause zisk_op = ZOP_BN254_COMPLEX_MUL : unit
function clause zisk_op_of_code (0xfe) = Some(ZOP_BN254_COMPLEX_MUL())
function clause zisk_exec_mem (ZOP_BN254_COMPLEX_MUL(), a, b, _) = {
  complex(a, b, BN254_P, 4, CMUL, "bn254_complex_mul");
  Some((zeros(), false))
}

/* BLS12-381's G1: y^2 = x^3 + 4; and F_p^2. */
union clause zisk_op = ZOP_BLS12_381_CURVE_ADD : unit
function clause zisk_op_of_code (0xe3) = Some(ZOP_BLS12_381_CURVE_ADD())
function clause zisk_exec_mem (ZOP_BLS12_381_CURVE_ADD(), a, b, _) = {
  curve_add(a, b, BLS12_381_P, 6, "bls12_381_curve_add");
  Some((zeros(), false))
}

union clause zisk_op = ZOP_BLS12_381_CURVE_DBL : unit
function clause zisk_op_of_code (0xe4) = Some(ZOP_BLS12_381_CURVE_DBL())
function clause zisk_exec_mem (ZOP_BLS12_381_CURVE_DBL(), _, b, _) = {
  curve_dbl(b, BLS12_381_P, 0, 6, "bls12_381_curve_dbl");
  Some((zeros(), false))
}

union clause zisk_op = ZOP_BLS12_381_COMPLEX_ADD : unit
function clause zisk_op_of_code (0xe5) = Some(ZOP_BLS12_381_COMPLEX_ADD())
function clause zisk_exec_mem (ZOP_BLS12_381_COMPLEX_ADD(), a, b, _) = {
  complex(a, b, BLS12_381_P, 6, CADD, "bls12_381_complex_add");
  Some((zeros(), false))
}

union clause zisk_op = ZOP_BLS12_381_COMPLEX_SUB : unit
function clause zisk_op_of_code (0xe6) = Some(ZOP_BLS12_381_COMPLEX_SUB())
function clause zisk_exec_mem (ZOP_BLS12_381_COMPLEX_SUB(), a, b, _) = {
  complex(a, b, BLS12_381_P, 6, CSUB, "bls12_381_complex_sub");
  Some((zeros(), false))
}

union clause zisk_op = ZOP_BLS12_381_COMPLEX_MUL : unit
function clause zisk_op_of_code (0xe7) = Some(ZOP_BLS12_381_COMPLEX_MUL())
function clause zisk_exec_mem (ZOP_BLS12_381_COMPLEX_MUL(), a, b, _) = {
  complex(a, b, BLS12_381_P, 6, CMUL, "bls12_381_complex_mul");
  Some((zeros(), false))
}

/* Twisted Edwards A x^2 + y^2 = 1 + D x^2 y^2: the unified addition, with
 * t = D x1 x2 y1 y2,
 *   x3 = (x1 y2 + y1 x2) / (1 + t),   y3 = (y1 y2 - A x1 x2) / (1 - t). */
val edwards_add : (int, int, int, int, int, int, int) -> (int, int)
function edwards_add(p, coef_a, coef_d, x1, y1, x2, y2) = {
  let t = (coef_d * x1 * x2 * y1 * y2) % p;
  (fdiv(x1 * y2 + y1 * x2, 1 + t, p), fdiv(y1 * y2 - coef_a * x1 * x2, 1 - t, p))
}

/* ed25519: A = -1, D = -121665 / 121666. Its operands are reduced mod p
 * rather than required canonical (ed25519_field_from_limbs). */
let ED25519_D : int = 37095705934669439343138083508754565189542113879843219016388785533085940283555

union clause zisk_op = ZOP_ED25519_ADD : unit
function clause zisk_op_of_code (0xe1) = Some(ZOP_ED25519_ADD())
function clause zisk_exec_mem (ZOP_ED25519_ADD(), a, b, _) = {
  check_aligned(a, "ed25519_add");
  check_aligned(b, "ed25519_add");
  let p = ED25519_P;
  let x1 = read_uint(a, 4) % p; let y1 = read_uint(a + 32, 4) % p;
  let x2 = read_uint(b, 4) % p; let y2 = read_uint(b + 32, 4) % p;
  write_pair(a, 4, edwards_add(p, -1, ED25519_D, x1, y1, x2, y2));
  Some((zeros(), false))
}

/* Baby Jubjub, over BN254's scalar field: A = 168700, D = 168696. */
union clause zisk_op = ZOP_BABYJUBJUB_ADD : unit
function clause zisk_op_of_code (0xed) = Some(ZOP_BABYJUBJUB_ADD())
function clause zisk_exec_mem (ZOP_BABYJUBJUB_ADD(), a, b, _) = {
  check_aligned(a, "babyjubjub_add");
  check_aligned(b, "babyjubjub_add");
  let (x1, y1) = read_pair(a, 4, BN254_R);
  let (x2, y2) = read_pair(b, 4, BN254_R);
  write_pair(a, 4, edwards_add(BN254_R, 168700, 168696, x1, y1, x2, y2));
  Some((zeros(), false))
}
