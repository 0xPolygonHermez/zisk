//! Checks the generated chunk equations of every twisted-Edwards curve against an honest witness:
//! every equation must close its carry chain to zero, with carries and quotients inside the AIR
//! ranges. Unlike the executor tests, inputs are fed as given (non-canonical too), as the AIR sees
//! them from memory.

use num_bigint::BigInt;
use num_traits::{One, Signed, Zero};

use super::ed25519_a::Ed25519A;
use super::ed25519_b::Ed25519B;
use super::ed25519_dt::Ed25519Dt;
use super::ed25519_n::Ed25519N;
use super::ed25519_t::Ed25519T;
use super::ed25519_x3::Ed25519X3;
use super::ed25519_y3::Ed25519Y3;
use super::{
    BabyJubJubA, BabyJubJubB, BabyJubJubDt, BabyJubJubN, BabyJubJubT, BabyJubJubX3, BabyJubJubY3,
};
use crate::babyjubjub_constants::{
    BABYJUBJUB_A, BABYJUBJUB_D, BABYJUBJUB_PRIME, BABYJUBJUB_PRIME_CHUNKS,
};
use crate::ed25519_constants::{ED25519_D, ED25519_PRIME, ED25519_PRIME_CHUNKS};
use zisk_precomp_helpers::bigint_to_16_chunks;

const COLS: u8 = 32;
// Must match the offsets in twisted_edwards_generator.rs.
const OFFSET: i64 = 0x10;
// Must match CARRY_BITS / Q_HSC_BITS in twisted_edwards.pil.
const CARRY_MIN: i64 = -((1 << 22) - 1);
const CARRY_MAX: i64 = 1 << 22;
const Q_HSC_MAX: i64 = (1 << 22) - 1;

type C = [i64; 16];

/// One curve: its parameters and the 7 generated equations (A, B, N, T, DT, X3, Y3).
struct Curve {
    p: BigInt,
    /// `a` as it appears in EQ_Y3 (signed: -1 for Ed25519), not reduced mod p.
    a: BigInt,
    d: BigInt,
    y3_offset: i64,
    /// A point on the curve (not the identity).
    base: (BigInt, BigInt),
    eq_a: fn(u8, &C, &C, &C, &C) -> i64,
    eq_b: fn(u8, &C, &C, &C, &C) -> i64,
    eq_n: fn(u8, &C, &C, &C, &C, &C, &C) -> i64,
    eq_t: fn(u8, &C, &C, &C, &C) -> i64,
    eq_dt: fn(u8, &C, &C, &C) -> i64,
    eq_x3: fn(u8, &C, &C, &C, &C) -> i64,
    eq_y3: fn(u8, &C, &C, &C, &C, &C) -> i64,
}

fn hex(s: &str) -> BigInt {
    BigInt::parse_bytes(s.as_bytes(), 16).unwrap()
}

fn dec(s: &str) -> BigInt {
    BigInt::parse_bytes(s.as_bytes(), 10).unwrap()
}

fn babyjubjub() -> Curve {
    Curve {
        p: hex(BABYJUBJUB_PRIME),
        a: BigInt::from(BABYJUBJUB_A),
        d: BigInt::from(BABYJUBJUB_D),
        y3_offset: 0x40000,
        // circomlib test/babyjub.js "Should add two points", p1.
        base: (
            dec("17777552123799933955779906779655732241715742912184938656739573121738514868268"),
            dec("2626589144620713026669568689430873010625803728049924121243784502389097019475"),
        ),
        eq_a: BabyJubJubA::calculate,
        eq_b: BabyJubJubB::calculate,
        eq_n: BabyJubJubN::calculate,
        eq_t: BabyJubJubT::calculate,
        eq_dt: BabyJubJubDt::calculate,
        eq_x3: BabyJubJubX3::calculate,
        eq_y3: BabyJubJubY3::calculate,
    }
}

fn ed25519() -> Curve {
    Curve {
        p: hex(ED25519_PRIME),
        a: BigInt::from(-1),
        d: hex(ED25519_D),
        y3_offset: OFFSET,
        // RFC 8032 base point B.
        base: (
            dec("15112221349535400772501151409588531511454012693041857206046113283949847762202"),
            dec("46316835694926478169428394003475163141307993866256225615783033603165251855960"),
        ),
        eq_a: Ed25519A::calculate,
        eq_b: Ed25519B::calculate,
        eq_n: Ed25519N::calculate,
        eq_t: Ed25519T::calculate,
        eq_dt: Ed25519Dt::calculate,
        eq_x3: Ed25519X3::calculate,
        eq_y3: Ed25519Y3::calculate,
    }
}

fn md(x: &BigInt, p: &BigInt) -> BigInt {
    ((x % p) + p) % p
}

fn inv(x: &BigInt, p: &BigInt) -> BigInt {
    x.modpow(&(p - BigInt::from(2)), p)
}

type Point = (BigInt, BigInt);

fn identity() -> Point {
    (BigInt::zero(), BigInt::one())
}

/// Reference complete addition.
fn add(c: &Curve, p1: &Point, p2: &Point) -> Point {
    let p = &c.p;
    let ((x1, y1), (x2, y2)) = (p1, p2);
    let tau = md(&(&c.d * x1 * x2 * y1 * y2), p);
    let x3 = md(&((x1 * y2 + y1 * x2) * inv(&md(&(BigInt::one() + &tau), p), p)), p);
    let y3 = md(&((y1 * y2 - &c.a * x1 * x2) * inv(&md(&(BigInt::one() - &tau), p), p)), p);
    (x3, y3)
}

fn on_curve(c: &Curve, (x, y): &Point) -> bool {
    let (x2, y2) = (x * x, y * y);
    md(&(&c.a * &x2 + &y2 - BigInt::one() - &c.d * &x2 * &y2), &c.p).is_zero()
}

fn neg(c: &Curve, (x, y): &Point) -> Point {
    (md(&-x, &c.p), y.clone())
}

fn chunks(v: &BigInt) -> C {
    assert!(!v.is_negative(), "negative witness value {v}");
    let mut c = [0i64; 16];
    bigint_to_16_chunks(v, &mut c);
    for (i, &x) in c.iter().enumerate() {
        let max = if i == 15 { Q_HSC_MAX } else { 0xFFFF };
        assert!((0..=max).contains(&x), "chunk {i} = {x} out of range for {v}");
    }
    c
}

fn quotient_plus(lhs: &BigInt, p: &BigInt) -> BigInt {
    assert!((lhs % p).is_zero());
    lhs / p + OFFSET
}

/// Builds the witness for p1 + p2 exactly as the AIR expects it (inputs taken as given, possibly
/// non-canonical; scratch values and outputs reduced) and checks all 7 carry chains.
fn check(c: &Curve, p1: &Point, p2: &Point) {
    check_with(c, p1, p2, 0);
}

/// `y3_delta` perturbs the claimed y3 (negative test).
fn check_with(c: &Curve, (x1, y1): &Point, (x2, y2): &Point, y3_delta: i64) {
    let p = &c.p;
    let a = md(&(x1 * x2), p);
    let b = md(&(y1 * y2), p);
    let n = md(&(x1 * y2 + y1 * x2), p);
    let t = md(&(&a * &b), p);
    let dt = md(&(&c.d * &t), p);
    let x3 = md(&(&n * inv(&md(&(BigInt::one() + &dt), p), p)), p);
    let y3 = md(&((&b - &c.a * &a) * inv(&md(&(BigInt::one() - &dt), p), p)), p) + y3_delta;

    let qa = quotient_plus(&(x1 * x2 - &a), p);
    let qb = quotient_plus(&(y1 * y2 - &b), p);
    let qn = quotient_plus(&(x1 * y2 + y1 * x2 - &n), p);
    let qt = quotient_plus(&(&a * &b - &t), p);
    let qdt = quotient_plus(&(&c.d * &t - &dt), p);
    let qx = quotient_plus(&(&x3 + &x3 * &dt - &n), p);
    // Minus convention. A wrong y3 has no exact quotient: use the truncated one so only the AIR
    // check can reject it.
    let y3_lhs = &y3 - &y3 * &dt - &b + &c.a * &a;
    assert!(y3_delta != 0 || (&y3_lhs % p).is_zero());
    let qy = BigInt::from(c.y3_offset) - &y3_lhs / p;

    let (cx1, cy1, cx2, cy2) = (chunks(x1), chunks(y1), chunks(x2), chunks(y2));
    let (ca, cb, cn, ct, cdt, cx3, cy3) =
        (chunks(&a), chunks(&b), chunks(&n), chunks(&t), chunks(&dt), chunks(&x3), chunks(&y3));
    let (cqa, cqb, cqn, cqt) = (chunks(&qa), chunks(&qb), chunks(&qn), chunks(&qt));
    let (cqdt, cqx, cqy) = (chunks(&qdt), chunks(&qx), chunks(&qy));

    let mut carry = [0i64; 7];
    for icol in 0..COLS {
        let eq = [
            (c.eq_a)(icol, &cx1, &cx2, &ca, &cqa),
            (c.eq_b)(icol, &cy1, &cy2, &cb, &cqb),
            (c.eq_n)(icol, &cx1, &cy2, &cy1, &cx2, &cn, &cqn),
            (c.eq_t)(icol, &ca, &cb, &ct, &cqt),
            (c.eq_dt)(icol, &ct, &cdt, &cqdt),
            (c.eq_x3)(icol, &cx3, &cdt, &cn, &cqx),
            (c.eq_y3)(icol, &cy3, &cdt, &cb, &ca, &cqy),
        ];
        for ieq in 0..7 {
            let value = eq[ieq] + carry[ieq];
            if icol == COLS - 1 {
                assert_eq!(value, 0, "eq{ieq} does not close (icol {icol})");
            } else {
                assert_eq!(value % 0x10000, 0, "eq{ieq} residue at icol {icol}");
                carry[ieq] = value / 0x10000;
                assert!(
                    (CARRY_MIN..=CARRY_MAX).contains(&carry[ieq]),
                    "eq{ieq} carry {} out of range at icol {icol}",
                    carry[ieq]
                );
            }
        }
    }
}

/// identity, k*base for k = 1..8, and -base.
fn sample_points(c: &Curve) -> Vec<Point> {
    assert!(on_curve(c, &c.base));
    let mut pts = vec![identity()];
    for i in 0..8 {
        let next = add(c, &pts[i], &c.base);
        assert!(on_curve(c, &next));
        pts.push(next);
    }
    pts.push(neg(c, &c.base));
    // Sanity of the reference: B + (-B) = identity, B + B = 2B.
    assert_eq!(add(c, &c.base, &neg(c, &c.base)), identity());
    assert_eq!(add(c, &c.base, &c.base), pts[2]);
    pts
}

fn check_honest_witness(c: &Curve) {
    let pts = sample_points(c);
    // Inputs are not alias-checked by the AIR: x + k*p (< 2^256) must also be provable. Use the
    // largest k that fits for every coordinate.
    let max_k = ((BigInt::one() << 256) - BigInt::one() - &c.p) / &c.p;
    let alias = |(x, y): &Point| (x + &max_k * &c.p, y + &max_k * &c.p);
    for p1 in &pts {
        for p2 in &pts {
            check(c, p1, p2);
            check(c, &alias(p1), &alias(p2));
        }
    }
}

fn check_worst_case_inputs(c: &Curve) {
    // All-ones 256-bit inputs maximise the quotients of the input products.
    let max: BigInt = (BigInt::one() << 256) - 1;
    check(c, &(max.clone(), max.clone()), &(max.clone(), max));
}

#[test]
fn babyjubjub_prime_chunks_match() {
    assert_eq!(chunks(&hex(BABYJUBJUB_PRIME)), BABYJUBJUB_PRIME_CHUNKS);
}

#[test]
fn babyjubjub_reference_matches_circomlib_vector() {
    // circomlib test/babyjub.js "Should add two points".
    let c = babyjubjub();
    let p2 = (
        dec("16540640123574156134436876038791482806971768689494387082833631921987005038935"),
        dec("20819045374670962167435360035096875258406992893633759881276124905556507972311"),
    );
    let expected = (
        dec("7916061937171219682591368294088513039687205273691143098332585753343424131937"),
        dec("14035240266687799601661095864649209771790948434046947201833777492504781204499"),
    );
    assert_eq!(add(&c, &c.base, &p2), expected);
    check(&c, &c.base, &p2);
}

#[test]
fn babyjubjub_equations_close_on_honest_witness() {
    check_honest_witness(&babyjubjub());
}

#[test]
fn babyjubjub_equations_worst_case_inputs() {
    check_worst_case_inputs(&babyjubjub());
}

#[test]
#[should_panic(expected = "eq6")]
fn babyjubjub_equations_reject_wrong_y3() {
    let c = babyjubjub();
    check_with(&c, &c.base, &c.base, 1);
}

#[test]
fn ed25519_prime_chunks_match() {
    let p = hex(ED25519_PRIME);
    assert_eq!(chunks(&p), ED25519_PRIME_CHUNKS);
    assert_eq!(p, (BigInt::one() << 255) - 19);
}

#[test]
fn ed25519_equations_close_on_honest_witness() {
    check_honest_witness(&ed25519());
}

#[test]
fn ed25519_equations_worst_case_inputs() {
    check_worst_case_inputs(&ed25519());
}

#[test]
#[should_panic(expected = "eq6")]
fn ed25519_equations_reject_wrong_y3() {
    let c = ed25519();
    check_with(&c, &c.base, &c.base, 1);
}
