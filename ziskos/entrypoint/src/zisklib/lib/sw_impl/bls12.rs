//! Software fallback for BLS12-381 operations using the blst crate (non-hints, non-zkVM builds only).
//!
//! The bulk of this file is copied verbatim from revm-precompile-32.1.0:
//!   - `src/bls12_381/blst.rs`               (blst wrapper helpers + byte-oriented API)
//!   - `src/bls12_381/pairing_common.rs`      (pairing_check_bytes_generic)
//!   - `src/kzg_point_evaluation/blst.rs`     (KZG verify_kzg_proof and helpers)
//!   - `src/bls12_381.rs`                     (type aliases G1Point / G2Point / …)
//!   - `src/bls12_381_const.rs`               (numeric constants + TRUSTED_SETUP_TAU_G2_BYTES)
//!
//! Adaptations made (minimal):
//!   - revm-internal imports replaced with local definitions
//!   - `crate::PrecompileError` → local `PrecompileError`
//!   - `primitives::OnceLock` → `std::sync::OnceLock`
//!   - `super::pairing_common::pairing_check_bytes_generic` → inlined in same file
//!   - `verify_kzg_proof` from kzg module renamed `kzg_verify_kzg_proof` to avoid conflict
//!
//! Public wrapper functions at the bottom adapt the flat byte-array API used by zkvm_accelerators.rs.

use blst::{
    blst_bendian_from_fp, blst_final_exp, blst_fp, blst_fp12, blst_fp12_is_one, blst_fp12_mul,
    blst_fp2, blst_fp_from_bendian, blst_map_to_g1, blst_map_to_g2, blst_miller_loop, blst_p1,
    blst_p1_add_or_double_affine, blst_p1_affine, blst_p1_affine_in_g1, blst_p1_affine_on_curve,
    blst_p1_from_affine, blst_p1_mult, blst_p1_to_affine, blst_p2, blst_p2_add_or_double_affine,
    blst_p2_affine, blst_p2_affine_in_g2, blst_p2_affine_on_curve, blst_p2_from_affine,
    blst_p2_mult, blst_p2_to_affine, blst_scalar, blst_scalar_from_bendian, MultiPoint,
};
use std::sync::OnceLock;
use std::vec::Vec;

// ============================================================
// Type aliases — revm-precompile-32.1.0/src/bls12_381.rs:19-32
// ============================================================

/// G1 point represented as two field elements (x, y coordinates)
type G1Point = ([u8; FP_LENGTH], [u8; FP_LENGTH]);
/// G2 point represented as four field elements (x0, x1, y0, y1 coordinates)
type G2Point = ([u8; FP_LENGTH], [u8; FP_LENGTH], [u8; FP_LENGTH], [u8; FP_LENGTH]);
/// G1 point and scalar pair for MSM operations
type G1PointScalar = (G1Point, [u8; SCALAR_LENGTH]);
/// G2 point and scalar pair for MSM operations
type G2PointScalar = (G2Point, [u8; SCALAR_LENGTH]);
type PairingPair = (G1Point, G2Point);

// ============================================================
// Constants — revm-precompile-32.1.0/src/bls12_381_const.rs:100-139,187-189
// ============================================================

/// FP_LENGTH specifies the number of bytes needed to represent an
/// Fp element. This is an element in the base field of BLS12-381.
///
/// Note: The base field is used to define G1 and G2 elements.
const FP_LENGTH: usize = 48;

/// G1_LENGTH specifies the number of bytes needed to represent a G1 element.
///
/// Note: A G1 element contains 2 Fp elements.
const G1_LENGTH: usize = 2 * FP_LENGTH;

/// G2_LENGTH specifies the number of bytes needed to represent a G2 element.
///
/// Note: A G2 element contains 2 Fp^2 elements.
const G2_LENGTH: usize = 4 * FP_LENGTH;

/// SCALAR_LENGTH specifies the number of bytes needed to represent an Fr element.
/// This is an element in the scalar field of BLS12-381.
///
/// Note: Since it is already 32 byte aligned, there is no padded version of this constant.
const SCALAR_LENGTH: usize = 32;
/// SCALAR_LENGTH_BITS specifies the number of bits needed to represent an Fr element.
/// This is an element in the scalar field of BLS12-381.
const SCALAR_LENGTH_BITS: usize = SCALAR_LENGTH * 8;

/// The trusted setup G2 point `[τ]₂` from the Ethereum KZG ceremony (compressed format)
/// Taken from: <https://github.com/ethereum/consensus-specs/blob/adc514a1c29532ebc1a67c71dc8741a2fdac5ed4/presets/mainnet/trusted_setups/trusted_setup_4096.json#L8200C6-L8200C200>
/// (revm-precompile-32.1.0/src/bls12_381_const.rs:187-189, hex decoded to byte literal)
const TRUSTED_SETUP_TAU_G2_BYTES: [u8; 96] = [
    0xb5, 0xbf, 0xd7, 0xdd, 0x8c, 0xde, 0xb1, 0x28, 0x84, 0x3b, 0xc2, 0x87, 0x23, 0x0a, 0xf3, 0x89,
    0x26, 0x18, 0x70, 0x75, 0xcb, 0xfb, 0xef, 0xa8, 0x10, 0x09, 0xa2, 0xce, 0x61, 0x5a, 0xc5, 0x3d,
    0x29, 0x14, 0xe5, 0x87, 0x0c, 0xb4, 0x52, 0xd2, 0xaf, 0xaa, 0xab, 0x24, 0xf3, 0x49, 0x9f, 0x72,
    0x18, 0x5c, 0xbf, 0xee, 0x53, 0x49, 0x27, 0x14, 0x73, 0x44, 0x29, 0xb7, 0xb3, 0x86, 0x08, 0xe2,
    0x39, 0x26, 0xc9, 0x11, 0xcc, 0xec, 0xea, 0xc9, 0xa3, 0x68, 0x51, 0x47, 0x7b, 0xa4, 0xc6, 0x0b,
    0x08, 0x70, 0x41, 0xde, 0x62, 0x10, 0x00, 0xed, 0xc9, 0x8e, 0xda, 0xda, 0x20, 0xc1, 0xde, 0xf2,
];

// ============================================================
// Error type (subset of revm's PrecompileError used in this module)
// ============================================================

#[derive(Debug)]
enum PrecompileError {
    NonCanonicalFp,
    Bls12381G1NotOnCurve,
    Bls12381G1NotInSubgroup,
    Bls12381G2NotOnCurve,
    Bls12381G2NotInSubgroup,
    Bls12381ScalarInputLength,
    KzgInvalidG1Point,
    KzgG1PointNotOnCurve,
    KzgG1PointNotInSubgroup,
}

// ============================================================
// Copied verbatim from revm-precompile-32.1.0/src/bls12_381/blst.rs
// ============================================================

// Big-endian non-Montgomery form.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:20-24
const MODULUS_REPR: [u8; 48] = [
    0x1a, 0x01, 0x11, 0xea, 0x39, 0x7f, 0xe6, 0x9a, 0x4b, 0x1b, 0xa7, 0xb6, 0x43, 0x4b, 0xac, 0xd7,
    0x64, 0x77, 0x4b, 0x84, 0xf3, 0x85, 0x12, 0xbf, 0x67, 0x30, 0xd2, 0xa0, 0xf6, 0xb0, 0xf6, 0x24,
    0x1e, 0xab, 0xff, 0xfe, 0xb1, 0x53, 0xff, 0xff, 0xb9, 0xfe, 0xff, 0xff, 0xff, 0xff, 0xaa, 0xab,
];

// revm-precompile-32.1.0/src/bls12_381/blst.rs:26-32
#[inline]
fn p1_to_affine(p: &blst_p1) -> blst_p1_affine {
    let mut p_affine = blst_p1_affine::default();
    // SAFETY: both inputs are valid blst types
    unsafe { blst_p1_to_affine(&mut p_affine, p) };
    p_affine
}

// revm-precompile-32.1.0/src/bls12_381/blst.rs:34-40
#[inline]
fn p1_from_affine(p_affine: &blst_p1_affine) -> blst_p1 {
    let mut p = blst_p1::default();
    // SAFETY: both inputs are valid blst types
    unsafe { blst_p1_from_affine(&mut p, p_affine) };
    p
}

// revm-precompile-32.1.0/src/bls12_381/blst.rs:42-48
#[inline]
fn p1_add_or_double(p: &blst_p1, p_affine: &blst_p1_affine) -> blst_p1 {
    let mut result = blst_p1::default();
    // SAFETY: all inputs are valid blst types
    unsafe { blst_p1_add_or_double_affine(&mut result, p, p_affine) };
    result
}

// revm-precompile-32.1.0/src/bls12_381/blst.rs:50-56
#[inline]
fn p2_to_affine(p: &blst_p2) -> blst_p2_affine {
    let mut p_affine = blst_p2_affine::default();
    // SAFETY: both inputs are valid blst types
    unsafe { blst_p2_to_affine(&mut p_affine, p) };
    p_affine
}

// revm-precompile-32.1.0/src/bls12_381/blst.rs:58-64
#[inline]
fn p2_from_affine(p_affine: &blst_p2_affine) -> blst_p2 {
    let mut p = blst_p2::default();
    // SAFETY: both inputs are valid blst types
    unsafe { blst_p2_from_affine(&mut p, p_affine) };
    p
}

// revm-precompile-32.1.0/src/bls12_381/blst.rs:66-72
#[inline]
fn p2_add_or_double(p: &blst_p2, p_affine: &blst_p2_affine) -> blst_p2 {
    let mut result = blst_p2::default();
    // SAFETY: all inputs are valid blst types
    unsafe { blst_p2_add_or_double_affine(&mut result, p, p_affine) };
    result
}

/// p1_add_affine adds two G1 points in affine form, returning the result in affine form
///
/// Note: `a` and `b` can be the same, ie this method is safe to call if one wants
/// to essentially double a point
// revm-precompile-32.1.0/src/bls12_381/blst.rs:74-88
#[inline]
fn p1_add_affine(a: &blst_p1_affine, b: &blst_p1_affine) -> blst_p1_affine {
    // Convert first point to Jacobian coordinates
    let a_jacobian = p1_from_affine(a);

    // Add second point (in affine) to first point (in Jacobian)
    let sum_jacobian = p1_add_or_double(&a_jacobian, b);

    // Convert result back to affine coordinates
    p1_to_affine(&sum_jacobian)
}

/// Add two G2 points in affine form, returning the result in affine form
// revm-precompile-32.1.0/src/bls12_381/blst.rs:90-101
#[inline]
fn p2_add_affine(a: &blst_p2_affine, b: &blst_p2_affine) -> blst_p2_affine {
    // Convert first point to Jacobian coordinates
    let a_jacobian = p2_from_affine(a);

    // Add second point (in affine) to first point (in Jacobian)
    let sum_jacobian = p2_add_or_double(&a_jacobian, b);

    // Convert result back to affine coordinates
    p2_to_affine(&sum_jacobian)
}

/// Performs a G1 scalar multiplication
///
/// Takes a G1 point in affine form and a scalar, and returns the result
/// of the scalar multiplication in affine form
///
/// Note: The scalar is expected to be in Big Endian format.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:103-128
#[inline]
fn p1_scalar_mul(p: &blst_p1_affine, scalar: &blst_scalar) -> blst_p1_affine {
    // Convert point to Jacobian coordinates
    let p_jacobian = p1_from_affine(p);

    let mut result = blst_p1::default();

    // SAFETY: all inputs are valid blst types
    unsafe { blst_p1_mult(&mut result, &p_jacobian, scalar.b.as_ptr(), scalar.b.len() * 8) };

    // Convert result back to affine coordinates
    p1_to_affine(&result)
}

/// Performs a G2 scalar multiplication
///
/// Takes a G2 point in affine form and a scalar, and returns the result
/// of the scalar multiplication in affine form
///
/// Note: The scalar is expected to be in Big Endian format.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:130-154
#[inline]
fn p2_scalar_mul(p: &blst_p2_affine, scalar: &blst_scalar) -> blst_p2_affine {
    // Convert point to Jacobian coordinates
    let p_jacobian = p2_from_affine(p);

    let mut result = blst_p2::default();
    // SAFETY: all inputs are valid blst types
    unsafe { blst_p2_mult(&mut result, &p_jacobian, scalar.b.as_ptr(), scalar.b.len() * 8) };

    // Convert result back to affine coordinates
    p2_to_affine(&result)
}

/// Performs multi-scalar multiplication (MSM) for G1 points
///
/// Takes a vector of G1 points and corresponding scalars, and returns their weighted sum
///
/// Note: This method assumes that `g1_points` does not contain any points at infinity.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:156-193
#[inline]
fn p1_msm(g1_points: Vec<blst_p1_affine>, scalars: Vec<blst_scalar>) -> blst_p1_affine {
    assert_eq!(
        g1_points.len(),
        scalars.len(),
        "number of scalars should equal the number of g1 points"
    );

    // When no inputs are given, we return the point at infinity.
    // This case can only trigger, if the initial MSM pairs
    // all had, either a zero scalar or the point at infinity.
    //
    // The precompile will return an error, if the initial input
    // was empty, in accordance with EIP-2537.
    if g1_points.is_empty() {
        return blst_p1_affine::default();
    }

    // When there is only a single point, we use a simpler scalar multiplication
    // procedure
    if g1_points.len() == 1 {
        return p1_scalar_mul(&g1_points[0], &scalars[0]);
    }

    // SAFETY: blst_scalar is repr(C) with a single `b: [u8; 32]` field.
    let scalars_bytes =
        unsafe { core::slice::from_raw_parts(scalars.as_ptr() as *const u8, scalars.len() * 32) };
    // Perform multi-scalar multiplication
    let multiexp = g1_points.mult(scalars_bytes, SCALAR_LENGTH_BITS);

    // Convert result back to affine coordinates
    p1_to_affine(&multiexp)
}

/// Performs multi-scalar multiplication (MSM) for G2 points
///
/// Takes a vector of G2 points and corresponding scalars, and returns their weighted sum
///
/// Note: Scalars are expected to be in Big Endian format.
/// This method assumes that `g2_points` does not contain any points at infinity.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:195-234
#[inline]
fn p2_msm(g2_points: Vec<blst_p2_affine>, scalars: Vec<blst_scalar>) -> blst_p2_affine {
    assert_eq!(
        g2_points.len(),
        scalars.len(),
        "number of scalars should equal the number of g2 points"
    );

    // When no inputs are given, we return the point at infinity.
    // This case can only trigger, if the initial MSM pairs
    // all had, either a zero scalar or the point at infinity.
    //
    // The precompile will return an error, if the initial input
    // was empty, in accordance with EIP-2537.
    if g2_points.is_empty() {
        return blst_p2_affine::default();
    }

    // When there is only a single point, we use a simpler scalar multiplication
    // procedure
    if g2_points.len() == 1 {
        return p2_scalar_mul(&g2_points[0], &scalars[0]);
    }

    // SAFETY: blst_scalar is repr(C) with a single `b: [u8; 32]` field.
    let scalars_bytes =
        unsafe { core::slice::from_raw_parts(scalars.as_ptr() as *const u8, scalars.len() * 32) };

    // Perform multi-scalar multiplication
    let multiexp = g2_points.mult(scalars_bytes, SCALAR_LENGTH_BITS);

    // Convert result back to affine coordinates
    p2_to_affine(&multiexp)
}

/// Maps a field element to a G1 point
///
/// Takes a field element (blst_fp) and returns the corresponding G1 point in affine form
// revm-precompile-32.1.0/src/bls12_381/blst.rs:236-251 (renamed map_fp_to_g1 → blst_map_fp_to_g1 to avoid conflict with public wrapper)
#[inline]
fn blst_map_fp_to_g1(fp: &blst_fp) -> blst_p1_affine {
    // Create a new G1 point in Jacobian coordinates
    let mut p = blst_p1::default();

    // Map the field element to a point on the curve
    // SAFETY: `p` and `fp` are blst values
    // Third argument is unused if null
    unsafe { blst_map_to_g1(&mut p, fp, core::ptr::null()) };

    // Convert to affine coordinates
    p1_to_affine(&p)
}

/// Maps a field element to a G2 point
///
/// Takes a field element (blst_fp2) and returns the corresponding G2 point in affine form
// revm-precompile-32.1.0/src/bls12_381/blst.rs:253-268 (renamed map_fp2_to_g2 → blst_map_fp2_to_g2 to avoid conflict with public wrapper)
#[inline]
fn blst_map_fp2_to_g2(fp2: &blst_fp2) -> blst_p2_affine {
    // Create a new G2 point in Jacobian coordinates
    let mut p = blst_p2::default();

    // Map the field element to a point on the curve
    // SAFETY: `p` and `fp2` are blst values
    // Third argument is unused if null
    unsafe { blst_map_to_g2(&mut p, fp2, core::ptr::null()) };

    // Convert to affine coordinates
    p2_to_affine(&p)
}

/// Computes a single miller loop for a given G1, G2 pair
// revm-precompile-32.1.0/src/bls12_381/blst.rs:270-279
#[inline]
fn compute_miller_loop(g1: &blst_p1_affine, g2: &blst_p2_affine) -> blst_fp12 {
    let mut result = blst_fp12::default();

    // SAFETY: All arguments are valid blst types
    unsafe { blst_miller_loop(&mut result, g2, g1) }

    result
}

/// multiply_fp12 multiplies two fp12 elements
// revm-precompile-32.1.0/src/bls12_381/blst.rs:281-290
#[inline]
fn multiply_fp12(a: &blst_fp12, b: &blst_fp12) -> blst_fp12 {
    let mut result = blst_fp12::default();

    // SAFETY: All arguments are valid blst types
    unsafe { blst_fp12_mul(&mut result, a, b) }

    result
}

/// final_exp computes the final exponentiation on an fp12 element
// revm-precompile-32.1.0/src/bls12_381/blst.rs:292-301
#[inline]
fn final_exp(f: &blst_fp12) -> blst_fp12 {
    let mut result = blst_fp12::default();

    // SAFETY: All arguments are valid blst types
    unsafe { blst_final_exp(&mut result, f) }

    result
}

/// is_fp12_one checks if an fp12 element equals
/// multiplicative identity element, one
// revm-precompile-32.1.0/src/bls12_381/blst.rs:303-309
#[inline]
fn is_fp12_one(f: &blst_fp12) -> bool {
    // SAFETY: argument is a valid blst type
    unsafe { blst_fp12_is_one(f) }
}

/// pairing_check performs a pairing check on a list of G1 and G2 point pairs and
/// returns true if the result is equal to the identity element.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:311-340 (renamed pairing_check → blst_pairing_check to avoid conflict with public wrapper)
#[inline]
fn blst_pairing_check(pairs: &[(blst_p1_affine, blst_p2_affine)]) -> bool {
    // When no inputs are given, we return true
    // This case can only trigger, if the initial pairing components
    // all had, either the G1 element as the point at infinity
    // or the G2 element as the point at infinity.
    //
    // The precompile will return an error, if the initial input
    // was empty, in accordance with EIP-2537.
    if pairs.is_empty() {
        return true;
    }
    // Compute the miller loop for the first pair
    let (first_g1, first_g2) = &pairs[0];
    let mut acc = compute_miller_loop(first_g1, first_g2);

    // For the remaining pairs, compute miller loop and multiply with the accumulated result
    for (g1, g2) in pairs.iter().skip(1) {
        let ml = compute_miller_loop(g1, g2);
        acc = multiply_fp12(&acc, &ml);
    }

    // Apply final exponentiation and check if result is 1
    let final_result = final_exp(&acc);

    // Check if the result is one (identity element)
    is_fp12_one(&final_result)
}

/// Encodes a G1 point in affine format into byte slice.
///
/// Note: The encoded bytes are in Big Endian format.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:342-350
fn encode_g1_point(input: &blst_p1_affine) -> [u8; G1_LENGTH] {
    let mut out = [0u8; G1_LENGTH];
    fp_to_bytes(&mut out[..FP_LENGTH], &input.x);
    fp_to_bytes(&mut out[FP_LENGTH..], &input.y);
    out
}

/// Encodes a single finite field element into byte slice.
///
/// Note: The encoded bytes are in Big Endian format.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:352-361
fn fp_to_bytes(out: &mut [u8], input: &blst_fp) {
    if out.len() != FP_LENGTH {
        return;
    }
    // SAFETY: Out length is checked previously, `input` is a blst value.
    unsafe { blst_bendian_from_fp(out.as_mut_ptr(), input) };
}

/// Returns a `blst_p1_affine` from the provided byte slices, which represent the x and y
/// affine coordinates of the point.
///
/// Note: Coordinates are expected to be in Big Endian format.
///
/// - If the x or y coordinate do not represent a canonical field element, an error is returned.
///   See [read_fp] for more information.
/// - If the point is not on the curve, an error is returned.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:363-392
fn decode_g1_on_curve(
    p0_x: &[u8; FP_LENGTH],
    p0_y: &[u8; FP_LENGTH],
) -> Result<blst_p1_affine, PrecompileError> {
    let out = blst_p1_affine { x: read_fp(p0_x)?, y: read_fp(p0_y)? };

    // From EIP-2537:
    //
    // Error cases:
    //
    // * An input is neither a point on the G1 elliptic curve nor the infinity point
    //
    // SAFETY: Out is a blst value.
    if unsafe { !blst_p1_affine_on_curve(&out) } {
        return Err(PrecompileError::Bls12381G1NotOnCurve);
    }

    Ok(out)
}

/// Extracts a G1 point in Affine format from the x and y coordinates.
///
/// Note: Coordinates are expected to be in Big Endian format.
/// By default, subgroup checks are performed.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:394-400
fn read_g1(x: &[u8; FP_LENGTH], y: &[u8; FP_LENGTH]) -> Result<blst_p1_affine, PrecompileError> {
    _extract_g1_input(x, y, true)
}
/// Extracts a G1 point in Affine format from the x and y coordinates
/// without performing a subgroup check.
///
/// Note: Coordinates are expected to be in Big Endian format.
/// Skipping subgroup checks can introduce security issues.
/// This method should only be called if:
///     - The EIP specifies that no subgroup check should be performed
///     - One can be certain that the point is in the correct subgroup.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:401-414
fn read_g1_no_subgroup_check(
    x: &[u8; FP_LENGTH],
    y: &[u8; FP_LENGTH],
) -> Result<blst_p1_affine, PrecompileError> {
    _extract_g1_input(x, y, false)
}
/// Extracts a G1 point in Affine format from the x and y coordinates.
///
/// Note: Coordinates are expected to be in Big Endian format.
/// This function will perform a G1 subgroup check if `subgroup_check` is set to `true`.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:415-444
fn _extract_g1_input(
    x: &[u8; FP_LENGTH],
    y: &[u8; FP_LENGTH],
    subgroup_check: bool,
) -> Result<blst_p1_affine, PrecompileError> {
    let out = decode_g1_on_curve(x, y)?;

    if subgroup_check {
        // NB: Subgroup checks
        //
        // Scalar multiplications, MSMs and pairings MUST perform a subgroup check.
        //
        // Implementations SHOULD use the optimized subgroup check method:
        //
        // https://eips.ethereum.org/assets/eip-2537/fast_subgroup_checks
        //
        // On any input that fail the subgroup check, the precompile MUST return an error.
        //
        // As endomorphism acceleration requires input on the correct subgroup, implementers MAY
        // use endomorphism acceleration.
        if unsafe { !blst_p1_affine_in_g1(&out) } {
            return Err(PrecompileError::Bls12381G1NotInSubgroup);
        }
    }
    Ok(out)
}

/// Encodes a G2 point in affine format into byte slice.
///
/// Note: The encoded bytes are in Big Endian format.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:446-456
fn encode_g2_point(input: &blst_p2_affine) -> [u8; G2_LENGTH] {
    let mut out = [0u8; G2_LENGTH];
    fp_to_bytes(&mut out[..FP_LENGTH], &input.x.fp[0]);
    fp_to_bytes(&mut out[FP_LENGTH..2 * FP_LENGTH], &input.x.fp[1]);
    fp_to_bytes(&mut out[2 * FP_LENGTH..3 * FP_LENGTH], &input.y.fp[0]);
    fp_to_bytes(&mut out[3 * FP_LENGTH..4 * FP_LENGTH], &input.y.fp[1]);
    out
}

/// Returns a `blst_p2_affine` from the provided byte slices, which represent the x and y
/// affine coordinates of the point.
///
/// Note: Coordinates are expected to be in Big Endian format.
///
/// - If the x or y coordinate do not represent a canonical field element, an error is returned.
///   See [read_fp2] for more information.
/// - If the point is not on the curve, an error is returned.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:458-489
fn decode_g2_on_curve(
    x1: &[u8; FP_LENGTH],
    x2: &[u8; FP_LENGTH],
    y1: &[u8; FP_LENGTH],
    y2: &[u8; FP_LENGTH],
) -> Result<blst_p2_affine, PrecompileError> {
    let out = blst_p2_affine { x: read_fp2(x1, x2)?, y: read_fp2(y1, y2)? };

    // From EIP-2537:
    //
    // Error cases:
    //
    // * An input is neither a point on the G2 elliptic curve nor the infinity point
    //
    // SAFETY: Out is a blst value.
    if unsafe { !blst_p2_affine_on_curve(&out) } {
        return Err(PrecompileError::Bls12381G2NotOnCurve);
    }

    Ok(out)
}

/// Creates a blst_fp2 element from two field elements.
///
/// Field elements are expected to be in Big Endian format.
/// Returns an error if either of the input field elements is not canonical.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:491-505
fn read_fp2(
    input_1: &[u8; FP_LENGTH],
    input_2: &[u8; FP_LENGTH],
) -> Result<blst_fp2, PrecompileError> {
    let fp_1 = read_fp(input_1)?;
    let fp_2 = read_fp(input_2)?;

    let fp2 = blst_fp2 { fp: [fp_1, fp_2] };

    Ok(fp2)
}
/// Extracts a G2 point in Affine format from the x and y coordinates.
///
/// Note: Coordinates are expected to be in Big Endian format.
/// By default, subgroup checks are performed.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:506-517
fn read_g2(
    a_x_0: &[u8; FP_LENGTH],
    a_x_1: &[u8; FP_LENGTH],
    a_y_0: &[u8; FP_LENGTH],
    a_y_1: &[u8; FP_LENGTH],
) -> Result<blst_p2_affine, PrecompileError> {
    _extract_g2_input(a_x_0, a_x_1, a_y_0, a_y_1, true)
}
/// Extracts a G2 point in Affine format from the x and y coordinates
/// without performing a subgroup check.
///
/// Note: Coordinates are expected to be in Big Endian format.
/// Skipping subgroup checks can introduce security issues.
/// This method should only be called if:
///     - The EIP specifies that no subgroup check should be performed
///     - One can be certain that the point is in the correct subgroup.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:518-533
fn read_g2_no_subgroup_check(
    a_x_0: &[u8; FP_LENGTH],
    a_x_1: &[u8; FP_LENGTH],
    a_y_0: &[u8; FP_LENGTH],
    a_y_1: &[u8; FP_LENGTH],
) -> Result<blst_p2_affine, PrecompileError> {
    _extract_g2_input(a_x_0, a_x_1, a_y_0, a_y_1, false)
}
/// Extracts a G2 point in Affine format from the x and y coordinates.
///
/// Note: Coordinates are expected to be in Big Endian format.
/// This function will perform a G2 subgroup check if `subgroup_check` is set to `true`.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:534-565
fn _extract_g2_input(
    a_x_0: &[u8; FP_LENGTH],
    a_x_1: &[u8; FP_LENGTH],
    a_y_0: &[u8; FP_LENGTH],
    a_y_1: &[u8; FP_LENGTH],
    subgroup_check: bool,
) -> Result<blst_p2_affine, PrecompileError> {
    let out = decode_g2_on_curve(a_x_0, a_x_1, a_y_0, a_y_1)?;

    if subgroup_check {
        // NB: Subgroup checks
        //
        // Scalar multiplications, MSMs and pairings MUST perform a subgroup check.
        //
        // Implementations SHOULD use the optimized subgroup check method:
        //
        // https://eips.ethereum.org/assets/eip-2537/fast_subgroup_checks
        //
        // On any input that fail the subgroup check, the precompile MUST return an error.
        //
        // As endomorphism acceleration requires input on the correct subgroup, implementers MAY
        // use endomorphism acceleration.
        if unsafe { !blst_p2_affine_in_g2(&out) } {
            return Err(PrecompileError::Bls12381G2NotInSubgroup);
        }
    }
    Ok(out)
}

/// Checks whether or not the input represents a canonical field element
/// returning the field element if successful.
///
/// Note: The field element is expected to be in big endian format.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:567-583
fn read_fp(input: &[u8; FP_LENGTH]) -> Result<blst_fp, PrecompileError> {
    if !is_valid_be(input) {
        return Err(PrecompileError::NonCanonicalFp);
    }
    let mut fp = blst_fp::default();
    // SAFETY: `input` has fixed length, and `fp` is a blst value.
    unsafe {
        // This performs the check for canonical field elements
        blst_fp_from_bendian(&mut fp, input.as_ptr());
    }

    Ok(fp)
}

/// Extracts a scalar from a 32 byte slice representation, decoding the input as a Big Endian
/// unsigned integer. If the input is not exactly 32 bytes long, an error is returned.
///
/// From [EIP-2537](https://eips.ethereum.org/EIPS/eip-2537):
/// * A scalar for the multiplication operation is encoded as 32 bytes by performing BigEndian
///   encoding of the corresponding (unsigned) integer.
///
/// We do not check that the scalar is a canonical Fr element, because the EIP specifies:
/// * The corresponding integer is not required to be less than or equal than main subgroup order
///   `q`.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:585-611
fn read_scalar(input: &[u8]) -> Result<blst_scalar, PrecompileError> {
    if input.len() != SCALAR_LENGTH {
        return Err(PrecompileError::Bls12381ScalarInputLength);
    }

    let mut out = blst_scalar::default();
    // SAFETY: `input` length is checked previously, out is a blst value.
    unsafe {
        // Note: We do not use `blst_scalar_fr_check` here because, from EIP-2537:
        //
        // * The corresponding integer is not required to be less than or equal than main subgroup
        // order `q`.
        blst_scalar_from_bendian(&mut out, input.as_ptr())
    };

    Ok(out)
}

/// Checks if the input is a valid big-endian representation of a field element.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:613-616
fn is_valid_be(input: &[u8; 48]) -> bool {
    *input < MODULUS_REPR
}

// Byte-oriented versions of the functions for external API compatibility

/// Performs point addition on two G1 points taking byte coordinates.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:620-639
#[inline]
fn p1_add_affine_bytes(a: G1Point, b: G1Point) -> Result<[u8; G1_LENGTH], PrecompileError> {
    let (a_x, a_y) = a;
    let (b_x, b_y) = b;
    // Parse first point
    let p1 = read_g1_no_subgroup_check(&a_x, &a_y)?;

    // Parse second point
    let p2 = read_g1_no_subgroup_check(&b_x, &b_y)?;

    // Perform addition
    let result = p1_add_affine(&p1, &p2);

    // Encode result
    Ok(encode_g1_point(&result))
}

/// Performs point addition on two G2 points taking byte coordinates.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:641-660
#[inline]
fn p2_add_affine_bytes(a: G2Point, b: G2Point) -> Result<[u8; G2_LENGTH], PrecompileError> {
    let (a_x_0, a_x_1, a_y_0, a_y_1) = a;
    let (b_x_0, b_x_1, b_y_0, b_y_1) = b;
    // Parse first point
    let p1 = read_g2_no_subgroup_check(&a_x_0, &a_x_1, &a_y_0, &a_y_1)?;

    // Parse second point
    let p2 = read_g2_no_subgroup_check(&b_x_0, &b_x_1, &b_y_0, &b_y_1)?;

    // Perform addition
    let result = p2_add_affine(&p1, &p2);

    // Encode result
    Ok(encode_g2_point(&result))
}

/// Maps a field element to a G1 point from bytes
// revm-precompile-32.1.0/src/bls12_381/blst.rs:662-670
#[inline]
fn map_fp_to_g1_bytes(fp_bytes: &[u8; FP_LENGTH]) -> Result<[u8; G1_LENGTH], PrecompileError> {
    let fp = read_fp(fp_bytes)?;
    let result = blst_map_fp_to_g1(&fp);
    Ok(encode_g1_point(&result))
}

/// Maps field elements to a G2 point from bytes
// revm-precompile-32.1.0/src/bls12_381/blst.rs:672-681
#[inline]
fn map_fp2_to_g2_bytes(
    fp2_x: &[u8; FP_LENGTH],
    fp2_y: &[u8; FP_LENGTH],
) -> Result<[u8; G2_LENGTH], PrecompileError> {
    let fp2 = read_fp2(fp2_x, fp2_y)?;
    let result = blst_map_fp2_to_g2(&fp2);
    Ok(encode_g2_point(&result))
}

/// Performs multi-scalar multiplication (MSM) for G1 points taking byte inputs.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:683-719
#[inline]
fn p1_msm_bytes(
    point_scalar_pairs: impl Iterator<Item = Result<G1PointScalar, PrecompileError>>,
) -> Result<[u8; G1_LENGTH], PrecompileError> {
    let (lower, _) = point_scalar_pairs.size_hint();
    let mut g1_points = Vec::with_capacity(lower);
    let mut scalars = Vec::with_capacity(lower);

    // Parse all points and scalars
    for pair_result in point_scalar_pairs {
        let ((x, y), scalar_bytes) = pair_result?;

        // NB: MSM requires subgroup check
        let point = read_g1(&x, &y)?;

        // Skip zero scalars after validating the point
        if scalar_bytes.iter().all(|&b| b == 0) {
            continue;
        }

        let scalar = read_scalar(&scalar_bytes)?;
        g1_points.push(point);
        scalars.push(scalar);
    }

    // Return point at infinity if no pairs were provided or all scalars were zero
    if g1_points.is_empty() {
        return Ok([0u8; G1_LENGTH]);
    }

    // Perform MSM
    let result = p1_msm(g1_points, scalars);

    // Encode result
    Ok(encode_g1_point(&result))
}

/// Performs multi-scalar multiplication (MSM) for G2 points taking byte inputs.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:721-757
#[inline]
fn p2_msm_bytes(
    point_scalar_pairs: impl Iterator<Item = Result<G2PointScalar, PrecompileError>>,
) -> Result<[u8; G2_LENGTH], PrecompileError> {
    let (lower, _) = point_scalar_pairs.size_hint();
    let mut g2_points = Vec::with_capacity(lower);
    let mut scalars = Vec::with_capacity(lower);

    // Parse all points and scalars
    for pair_result in point_scalar_pairs {
        let ((x_0, x_1, y_0, y_1), scalar_bytes) = pair_result?;

        // NB: MSM requires subgroup check
        let point = read_g2(&x_0, &x_1, &y_0, &y_1)?;

        // Skip zero scalars after validating the point
        if scalar_bytes.iter().all(|&b| b == 0) {
            continue;
        }

        let scalar = read_scalar(&scalar_bytes)?;
        g2_points.push(point);
        scalars.push(scalar);
    }

    // Return point at infinity if no pairs were provided or all scalars were zero
    if g2_points.is_empty() {
        return Ok([0u8; G2_LENGTH]);
    }

    // Perform MSM
    let result = p2_msm(g2_points, scalars);

    // Encode result
    Ok(encode_g2_point(&result))
}

/// pairing_check_bytes performs a pairing check on a list of G1 and G2 point pairs taking byte inputs.
// revm-precompile-32.1.0/src/bls12_381/blst.rs:759-763
#[inline]
fn pairing_check_bytes(pairs: &[PairingPair]) -> Result<bool, PrecompileError> {
    pairing_check_bytes_generic(pairs, read_g1, read_g2, blst_pairing_check)
}

// ============================================================
// Copied verbatim from revm-precompile-32.1.0/src/bls12_381/pairing_common.rs
// (super::PairingPair → PairingPair; crate::PrecompileError → PrecompileError)
// ============================================================

/// Shared implementation of `pairing_check_bytes`.
// revm-precompile-32.1.0/src/bls12_381/pairing_common.rs:12-61
#[inline]
fn pairing_check_bytes_generic<G1, G2, ReadG1, ReadG2, PairingCheckFn>(
    pairs: &[PairingPair],
    read_g1: ReadG1,
    read_g2: ReadG2,
    pairing_check_fn: PairingCheckFn,
) -> Result<bool, PrecompileError>
where
    ReadG1: Fn(&[u8; 48], &[u8; 48]) -> Result<G1, PrecompileError>,
    ReadG2: Fn(&[u8; 48], &[u8; 48], &[u8; 48], &[u8; 48]) -> Result<G2, PrecompileError>,
    PairingCheckFn: FnOnce(&[(G1, G2)]) -> bool,
{
    if pairs.is_empty() {
        return Ok(true);
    }

    let mut parsed_pairs = Vec::with_capacity(pairs.len());
    for ((g1_x, g1_y), (g2_x_0, g2_x_1, g2_y_0, g2_y_1)) in pairs {
        // Check if G1 point is zero (point at infinity)
        let g1_is_zero = g1_x.iter().all(|&b| b == 0) && g1_y.iter().all(|&b| b == 0);

        // Check if G2 point is zero (point at infinity)
        let g2_is_zero = g2_x_0.iter().all(|&b| b == 0)
            && g2_x_1.iter().all(|&b| b == 0)
            && g2_y_0.iter().all(|&b| b == 0)
            && g2_y_1.iter().all(|&b| b == 0);

        // Skip this pair if either point is at infinity as it's a no-op
        if g1_is_zero || g2_is_zero {
            // Still need to validate the non-zero point if one exists
            if !g1_is_zero {
                let _ = read_g1(g1_x, g1_y)?;
            }
            if !g2_is_zero {
                let _ = read_g2(g2_x_0, g2_x_1, g2_y_0, g2_y_1)?;
            }
            continue;
        }

        let g1_point = read_g1(g1_x, g1_y)?;
        let g2_point = read_g2(g2_x_0, g2_x_1, g2_y_0, g2_y_1)?;
        parsed_pairs.push((g1_point, g2_point));
    }

    // If all pairs were filtered out, return true (identity element)
    if parsed_pairs.is_empty() {
        return Ok(true);
    }

    Ok(pairing_check_fn(&parsed_pairs))
}

// ============================================================
// Copied verbatim from revm-precompile-32.1.0/src/kzg_point_evaluation/blst.rs
// Adaptations:
//   - imports replaced with local definitions (all helpers are in scope)
//   - `primitives::OnceLock` → `std::sync::OnceLock`
//   - `TRUSTED_SETUP_TAU_G2_BYTES` already defined locally above
//   - `verify_kzg_proof` renamed to `kzg_verify_kzg_proof` (avoids conflict with public wrapper)
// ============================================================

/// Verify KZG proof using BLST BLS12-381 implementation.
///
/// <https://github.com/ethereum/EIPs/blob/4d2a00692bb131366ede1a16eced2b0e25b1bf99/EIPS/eip-4844.md?plain=1#L203>
/// <https://github.com/ethereum/consensus-specs/blob/master/specs/deneb/polynomial-commitments.md#verify_kzg_proof_impl>
// revm-precompile-32.1.0/src/kzg_point_evaluation/blst.rs:20-65 (renamed verify_kzg_proof → kzg_verify_kzg_proof)
#[inline]
fn kzg_verify_kzg_proof(
    commitment: &[u8; 48],
    z: &[u8; 32],
    y: &[u8; 32],
    proof: &[u8; 48],
) -> bool {
    // Parse the commitment (G1 point)
    let Ok(commitment_point) = parse_g1_compressed(commitment) else {
        return false;
    };

    // Parse the proof (G1 point)
    let Ok(proof_point) = parse_g1_compressed(proof) else {
        return false;
    };

    // Parse z and y as field elements (Fr, scalar field)
    let Ok(z_scalar) = read_scalar_canonical(z) else {
        return false;
    };
    let Ok(y_scalar) = read_scalar_canonical(y) else {
        return false;
    };

    // Get the trusted setup G2 point [τ]₂
    let tau_g2 = get_trusted_setup_g2();

    // Get generators
    let g1 = get_g1_generator();
    let g2 = get_g2_generator();

    // Compute P_minus_y = commitment - [y]G₁
    let y_g1 = p1_scalar_mul(&g1, &y_scalar);
    let p_minus_y = p1_sub_affine(&commitment_point, &y_g1);

    // Compute X_minus_z = [τ]G₂ - [z]G₂
    let z_g2 = p2_scalar_mul(&g2, &z_scalar);
    let x_minus_z = p2_sub_affine(tau_g2, &z_g2);

    // Verify: P - y = Q * (X - z)
    // Using pairing check: e(P - y, -G₂) * e(proof, X - z) == 1
    let neg_g2 = p2_neg(&g2);

    blst_pairing_check(&[(p_minus_y, neg_g2), (proof_point, x_minus_z)])
}

/// Get the trusted setup G2 point `[τ]₂` from the Ethereum KZG ceremony.
/// This is g2_monomial_1 from trusted_setup_4096.json
// revm-precompile-32.1.0/src/kzg_point_evaluation/blst.rs:67-85
fn get_trusted_setup_g2() -> &'static blst_p2_affine {
    static TAU_G2: OnceLock<blst_p2_affine> = OnceLock::new();
    TAU_G2.get_or_init(|| {
        // For compressed G2, we need to decompress
        let mut g2_affine = blst_p2_affine::default();
        unsafe {
            // The compressed format has x coordinate and a flag bit for y
            // We use uncompress which handles this automatically
            let result =
                blst::blst_p2_uncompress(&mut g2_affine, TRUSTED_SETUP_TAU_G2_BYTES.as_ptr());
            if result != blst::BLST_ERROR::BLST_SUCCESS {
                panic!("Failed to deserialize trusted setup G2 point");
            }
        }
        g2_affine
    })
}

/// Get G1 generator point
// revm-precompile-32.1.0/src/kzg_point_evaluation/blst.rs:87-90
fn get_g1_generator() -> blst_p1_affine {
    unsafe { ::blst::BLS12_381_G1 }
}

/// Get G2 generator point
// revm-precompile-32.1.0/src/kzg_point_evaluation/blst.rs:92-95
fn get_g2_generator() -> blst_p2_affine {
    unsafe { ::blst::BLS12_381_G2 }
}

/// Parse a G1 point from compressed format (48 bytes)
// revm-precompile-32.1.0/src/kzg_point_evaluation/blst.rs:97-117
fn parse_g1_compressed(bytes: &[u8; 48]) -> Result<blst_p1_affine, PrecompileError> {
    let mut point = blst_p1_affine::default();
    unsafe {
        let result = blst::blst_p1_uncompress(&mut point, bytes.as_ptr());
        if result != blst::BLST_ERROR::BLST_SUCCESS {
            return Err(PrecompileError::KzgInvalidG1Point);
        }

        // Verify the point is on curve
        if !blst_p1_affine_on_curve(&point) {
            return Err(PrecompileError::KzgG1PointNotOnCurve);
        }

        // Verify the point is in the correct subgroup
        if !blst_p1_affine_in_g1(&point) {
            return Err(PrecompileError::KzgG1PointNotInSubgroup);
        }
    }
    Ok(point)
}

/// Read a scalar field element from bytes and verify it's canonical
// revm-precompile-32.1.0/src/kzg_point_evaluation/blst.rs:119-133
fn read_scalar_canonical(bytes: &[u8; 32]) -> Result<blst_scalar, PrecompileError> {
    let mut scalar = blst_scalar::default();

    // Read scalar from big endian bytes
    unsafe {
        blst_scalar_from_bendian(&mut scalar, bytes.as_ptr());
    }

    if unsafe { !blst::blst_scalar_fr_check(&scalar) } {
        return Err(PrecompileError::NonCanonicalFp);
    }

    Ok(scalar)
}

/// Subtract two G1 points in affine form
// revm-precompile-32.1.0/src/kzg_point_evaluation/blst.rs:135-147
fn p1_sub_affine(a: &blst_p1_affine, b: &blst_p1_affine) -> blst_p1_affine {
    // Convert first point to Jacobian
    let a_jacobian = p1_from_affine(a);

    // Negate second point
    let neg_b = p1_neg(b);

    // Add a + (-b)
    let result = p1_add_or_double(&a_jacobian, &neg_b);

    p1_to_affine(&result)
}

/// Subtract two G2 points in affine form
// revm-precompile-32.1.0/src/kzg_point_evaluation/blst.rs:149-161
fn p2_sub_affine(a: &blst_p2_affine, b: &blst_p2_affine) -> blst_p2_affine {
    // Convert first point to Jacobian
    let a_jacobian = p2_from_affine(a);

    // Negate second point
    let neg_b = p2_neg(b);

    // Add a + (-b)
    let result = p2_add_or_double(&a_jacobian, &neg_b);

    p2_to_affine(&result)
}

/// Negate a G1 point
// revm-precompile-32.1.0/src/kzg_point_evaluation/blst.rs:163-171
fn p1_neg(p: &blst_p1_affine) -> blst_p1_affine {
    // Convert to Jacobian, negate, convert back
    let mut p_jacobian = p1_from_affine(p);
    unsafe {
        ::blst::blst_p1_cneg(&mut p_jacobian, true);
    }
    p1_to_affine(&p_jacobian)
}

/// Negate a G2 point
// revm-precompile-32.1.0/src/kzg_point_evaluation/blst.rs:173-181
fn p2_neg(p: &blst_p2_affine) -> blst_p2_affine {
    // Convert to Jacobian, negate, convert back
    let mut p_jacobian = p2_from_affine(p);
    unsafe {
        ::blst::blst_p2_cneg(&mut p_jacobian, true);
    }
    p2_to_affine(&p_jacobian)
}

// ============================================================
// Public wrappers adapting the flat byte-array API used by zkvm_accelerators.rs
//
// The revm byte-oriented API uses coordinate tuples: G1Point = ([u8; 48], [u8; 48]).
// zkvm_accelerators.rs passes flat arrays: &[u8; 96] for G1, &[u8; 192] for G2,
// and raw pointers for MSM/pairing pair arrays.
// ============================================================

/// G1 point addition: two flat 96-byte points → 96-byte result.
pub fn g1_add(p1: &[u8; 96], p2: &[u8; 96]) -> Option<[u8; 96]> {
    let a: G1Point = (p1[..48].try_into().unwrap(), p1[48..].try_into().unwrap());
    let b: G1Point = (p2[..48].try_into().unwrap(), p2[48..].try_into().unwrap());
    p1_add_affine_bytes(a, b).ok()
}

/// G1 MSM: slice of (96-byte G1 point || 32-byte scalar) pairs → 96-byte result.
pub fn g1_msm(pairs: &[[u8; 128]]) -> Option<[u8; 96]> {
    let iter = pairs.iter().map(|pair| -> Result<G1PointScalar, PrecompileError> {
        let x: [u8; 48] = pair[0..48].try_into().unwrap();
        let y: [u8; 48] = pair[48..96].try_into().unwrap();
        let s: [u8; 32] = pair[96..128].try_into().unwrap();
        Ok(((x, y), s))
    });
    p1_msm_bytes(iter).ok()
}

/// G2 point addition: two flat 192-byte points → 192-byte result.
pub fn g2_add(p1: &[u8; 192], p2: &[u8; 192]) -> Option<[u8; 192]> {
    let split_g2 = |p: &[u8; 192]| -> G2Point {
        (
            p[0..48].try_into().unwrap(),
            p[48..96].try_into().unwrap(),
            p[96..144].try_into().unwrap(),
            p[144..192].try_into().unwrap(),
        )
    };
    p2_add_affine_bytes(split_g2(p1), split_g2(p2)).ok()
}

/// G2 MSM: slice of (192-byte G2 point || 32-byte scalar) pairs → 192-byte result.
pub fn g2_msm(pairs: &[[u8; 224]]) -> Option<[u8; 192]> {
    let iter = pairs.iter().map(|pair| -> Result<G2PointScalar, PrecompileError> {
        let x0: [u8; 48] = pair[0..48].try_into().unwrap();
        let x1: [u8; 48] = pair[48..96].try_into().unwrap();
        let y0: [u8; 48] = pair[96..144].try_into().unwrap();
        let y1: [u8; 48] = pair[144..192].try_into().unwrap();
        let s: [u8; 32] = pair[192..224].try_into().unwrap();
        Ok(((x0, x1, y0, y1), s))
    });
    p2_msm_bytes(iter).ok()
}

/// Pairing check: slice of (96-byte G1 || 192-byte G2) pairs → bool result.
pub fn pairing_check(pairs: &[[u8; 288]]) -> Option<bool> {
    let pairing_pairs: Vec<PairingPair> = pairs
        .iter()
        .map(|pair| {
            let g1x: [u8; 48] = pair[0..48].try_into().unwrap();
            let g1y: [u8; 48] = pair[48..96].try_into().unwrap();
            let g2x0: [u8; 48] = pair[96..144].try_into().unwrap();
            let g2x1: [u8; 48] = pair[144..192].try_into().unwrap();
            let g2y0: [u8; 48] = pair[192..240].try_into().unwrap();
            let g2y1: [u8; 48] = pair[240..288].try_into().unwrap();
            ((g1x, g1y), (g2x0, g2x1, g2y0, g2y1))
        })
        .collect();
    pairing_check_bytes(&pairing_pairs).ok()
}

/// Map Fp field element to G1 point: 48-byte input → 96-byte output.
pub fn map_fp_to_g1(field_element: &[u8; 48]) -> Option<[u8; 96]> {
    map_fp_to_g1_bytes(field_element).ok()
}

/// Map Fp2 field element to G2 point: 96-byte input → 192-byte output.
pub fn map_fp2_to_g2(field_element: &[u8; 96]) -> Option<[u8; 192]> {
    let fp2_x: &[u8; 48] = field_element[..48].try_into().unwrap();
    let fp2_y: &[u8; 48] = field_element[48..].try_into().unwrap();
    map_fp2_to_g2_bytes(fp2_x, fp2_y).ok()
}

/// Verify KZG proof.
///
/// Parameter order matches zkvm_accelerators.rs: z, y, commitment, proof.
/// (revm's `verify_kzg_proof` has commitment first; this wrapper reorders.)
pub fn verify_kzg_proof(
    z: &[u8; 32],
    y: &[u8; 32],
    commitment: &[u8; 48],
    proof: &[u8; 48],
) -> bool {
    kzg_verify_kzg_proof(commitment, z, y, proof)
}
