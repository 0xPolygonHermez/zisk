//! Software fallback for BN254 operations using the ark-bn254 crate (non-hints, non-zkVM builds only).
//!
//! The bulk of this file is copied verbatim from revm-precompile-32.1.0:
//!   - `src/bn254/arkworks.rs`   (all arithmetic helpers + g1_point_add/g1_point_mul/pairing_check)
//!   - `src/bn254.rs`            (constants FQ_LEN / SCALAR_LEN / FQ2_LEN / G1_LEN / G2_LEN)
//!
//! Adaptations made (minimal):
//!   - revm-internal imports replaced with local definitions
//!   - `crate::PrecompileError` → local `PrecompileError`
//!   - `super::{FQ2_LEN, FQ_LEN, G1_LEN, SCALAR_LEN}` → local constants
//!   - `pairing_check` (inner) renamed `arkworks_pairing_check` to avoid conflict with public wrapper
//!
//! Public wrapper functions at the bottom adapt the fixed-size byte-array API used by zkvm_accelerators.rs.

use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G1Projective, G2Affine};
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};
use ark_ff::{One, PrimeField, Zero};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use std::vec::Vec;

// ============================================================
// Constants — revm-precompile-32.1.0/src/bn254.rs:116-139
// ============================================================

/// FQ_LEN specifies the number of bytes needed to represent an
/// Fq element. This is an element in the base field of BN254.
///
/// Note: The base field is used to define G1 and G2 elements.
const FQ_LEN: usize = 32;

/// SCALAR_LEN specifies the number of bytes needed to represent an Fr element.
/// This is an element in the scalar field of BN254.
const SCALAR_LEN: usize = 32;

/// FQ2_LEN specifies the number of bytes needed to represent an
/// Fq^2 element.
///
/// Note: This is the quadratic extension of Fq, and by definition
/// means we need 2 Fq elements.
const FQ2_LEN: usize = 2 * FQ_LEN;

/// G1_LEN specifies the number of bytes needed to represent a G1 element.
///
/// Note: A G1 element contains 2 Fq elements.
const G1_LEN: usize = 2 * FQ_LEN;
/// G2_LEN specifies the number of bytes needed to represent a G2 element.
///
/// Note: A G2 element contains 2 Fq^2 elements.
#[allow(dead_code)]
const G2_LEN: usize = 2 * FQ2_LEN;

// ============================================================
// Error type (subset of revm's PrecompileError used in this module)
// ============================================================

#[derive(Debug)]
enum PrecompileError {
    Bn254FieldPointNotAMember,
    Bn254AffineGFailedToCreate,
}

// ============================================================
// Copied verbatim from revm-precompile-32.1.0/src/bn254/arkworks.rs
//  super::{FQ2_LEN, FQ_LEN, G1_LEN, SCALAR_LEN} → local constants;
//  pairing_check renamed arkworks_pairing_check to avoid conflict with public wrapper)
// ============================================================

/// Reads a single `Fq` field element from the input slice.
///
/// Takes a byte slice and attempts to interpret the first 32 bytes as an
/// elliptic curve field element. Returns an error if the bytes do not form
/// a valid field element.
///
/// # Panics
///
/// Panics if the input is not at least 32 bytes long.
// revm-precompile-32.1.0/src/bn254/arkworks.rs:21-32
#[inline]
fn read_fq(input_be: &[u8]) -> Result<Fq, PrecompileError> {
    assert_eq!(input_be.len(), FQ_LEN, "input must be {FQ_LEN} bytes");

    let mut input_le = [0u8; FQ_LEN];
    input_le.copy_from_slice(input_be);

    // Reverse in-place to convert from big-endian to little-endian.
    input_le.reverse();

    Fq::deserialize_uncompressed(&input_le[..])
        .map_err(|_| PrecompileError::Bn254FieldPointNotAMember)
}
/// Reads a Fq2 (quadratic extension field element) from the input slice.
///
/// Parses two consecutive Fq field elements as the real and imaginary parts
/// of an Fq2 element.
/// The second component is parsed before the first, ie if a we represent an
/// element in Fq2 as (x,y) -- `y` is parsed before `x`
///
/// # Panics
///
/// Panics if the input is not at least 64 bytes long.
// revm-precompile-32.1.0/src/bn254/arkworks.rs:43-49
#[inline]
fn read_fq2(input: &[u8]) -> Result<Fq2, PrecompileError> {
    let y = read_fq(&input[..FQ_LEN])?;
    let x = read_fq(&input[FQ_LEN..2 * FQ_LEN])?;

    Ok(Fq2::new(x, y))
}

/// Creates a new `G1` point from the given `x` and `y` coordinates.
///
/// Constructs a point on the G1 curve from its affine coordinates.
///
/// Note: The point at infinity which is represented as (0,0) is
/// handled specifically because `AffineG1` is not capable of
/// representing such a point.
/// In particular, when we convert from `AffineG1` to `G1`, the point
/// will be (0,0,1) instead of (0,1,0)
// revm-precompile-32.1.0/src/bn254/arkworks.rs:60-72
#[inline]
fn new_g1_point(px: Fq, py: Fq) -> Result<G1Affine, PrecompileError> {
    if px.is_zero() && py.is_zero() {
        Ok(G1Affine::zero())
    } else {
        // We cannot use `G1Affine::new` because that triggers an assert if the point is not on the curve.
        let point = G1Affine::new_unchecked(px, py);
        if !point.is_on_curve() || !point.is_in_correct_subgroup_assuming_on_curve() {
            return Err(PrecompileError::Bn254AffineGFailedToCreate);
        }
        Ok(point)
    }
}

/// Creates a new `G2` point from the given Fq2 coordinates.
///
/// G2 points in BN254 are defined over a quadratic extension field Fq2.
/// This function takes two Fq2 elements representing the x and y coordinates
/// and creates a G2 point.
///
/// Note: The point at infinity which is represented as (0,0) is
/// handled specifically because `AffineG2` is not capable of
/// representing such a point.
/// In particular, when we convert from `AffineG2` to `G2`, the point
/// will be (0,0,1) instead of (0,1,0)
// revm-precompile-32.1.0/src/bn254/arkworks.rs:85-99
#[inline]
fn new_g2_point(x: Fq2, y: Fq2) -> Result<G2Affine, PrecompileError> {
    let point = if x.is_zero() && y.is_zero() {
        G2Affine::zero()
    } else {
        // We cannot use `G1Affine::new` because that triggers an assert if the point is not on the curve.
        let point = G2Affine::new_unchecked(x, y);
        if !point.is_on_curve() || !point.is_in_correct_subgroup_assuming_on_curve() {
            return Err(PrecompileError::Bn254AffineGFailedToCreate);
        }
        point
    };

    Ok(point)
}

/// Reads a G1 point from the input slice.
///
/// Parses a G1 point from a byte slice by reading two consecutive field elements
/// representing the x and y coordinates.
///
/// # Panics
///
/// Panics if the input is not at least 64 bytes long.
// revm-precompile-32.1.0/src/bn254/arkworks.rs:109-114
#[inline]
fn read_g1_point(input: &[u8]) -> Result<G1Affine, PrecompileError> {
    let px = read_fq(&input[0..FQ_LEN])?;
    let py = read_fq(&input[FQ_LEN..2 * FQ_LEN])?;
    new_g1_point(px, py)
}

/// Encodes a G1 point into a byte array.
///
/// Converts a G1 point in Jacobian coordinates to affine coordinates and
/// serializes the x and y coordinates as big-endian byte arrays.
///
/// Note: If the point is the point at infinity, this function returns
/// all zeroes.
// revm-precompile-32.1.0/src/bn254/arkworks.rs:123-147
#[inline]
fn encode_g1_point(point: G1Affine) -> [u8; G1_LEN] {
    let mut output = [0u8; G1_LEN];
    let Some((x, y)) = point.xy() else {
        return output;
    };

    let mut x_bytes = [0u8; FQ_LEN];
    x.serialize_uncompressed(&mut x_bytes[..]).expect("Failed to serialize x coordinate");

    let mut y_bytes = [0u8; FQ_LEN];
    y.serialize_uncompressed(&mut y_bytes[..]).expect("Failed to serialize x coordinate");

    // Convert to big endian by reversing the bytes.
    x_bytes.reverse();
    y_bytes.reverse();

    // Place x in the first half, y in the second half.
    output[0..FQ_LEN].copy_from_slice(&x_bytes);
    output[FQ_LEN..(FQ_LEN * 2)].copy_from_slice(&y_bytes);

    output
}

/// Reads a G2 point from the input slice.
///
/// Parses a G2 point from a byte slice by reading four consecutive Fq field elements
/// representing the two Fq2 coordinates (x and y) of the G2 point.
///
/// # Panics
///
/// Panics if the input is not at least 128 bytes long.
// revm-precompile-32.1.0/src/bn254/arkworks.rs:157-162
#[inline]
fn read_g2_point(input: &[u8]) -> Result<G2Affine, PrecompileError> {
    let ba = read_fq2(&input[0..FQ2_LEN])?;
    let bb = read_fq2(&input[FQ2_LEN..2 * FQ2_LEN])?;
    new_g2_point(ba, bb)
}

/// Reads a scalar from the input slice
///
/// Note: The scalar does not need to be canonical.
///
/// # Panics
///
/// If `input.len()` is not equal to [`SCALAR_LEN`].
// revm-precompile-32.1.0/src/bn254/arkworks.rs:171-180
#[inline]
fn read_scalar(input: &[u8]) -> Fr {
    assert_eq!(
        input.len(),
        SCALAR_LEN,
        "unexpected scalar length. got {}, expected {SCALAR_LEN}",
        input.len()
    );
    Fr::from_be_bytes_mod_order(input)
}

/// Performs point addition on two G1 points.
// revm-precompile-32.1.0/src/bn254/arkworks.rs:182-194
#[inline]
fn g1_point_add(p1_bytes: &[u8], p2_bytes: &[u8]) -> Result<[u8; 64], PrecompileError> {
    let p1 = read_g1_point(p1_bytes)?;
    let p2 = read_g1_point(p2_bytes)?;

    let p1_jacobian: G1Projective = p1.into();

    let p3 = p1_jacobian + p2;
    let output = encode_g1_point(p3.into_affine());

    Ok(output)
}

/// Performs a G1 scalar multiplication.
// revm-precompile-32.1.0/src/bn254/arkworks.rs:196-211
#[inline]
fn g1_point_mul(point_bytes: &[u8], fr_bytes: &[u8]) -> Result<[u8; 64], PrecompileError> {
    let p = read_g1_point(point_bytes)?;
    let fr = read_scalar(fr_bytes);

    let big_int = fr.into_bigint();
    let result = p.mul_bigint(big_int);

    let output = encode_g1_point(result.into_affine());

    Ok(output)
}

/// pairing_check performs a pairing check on a list of G1 and G2 point pairs and
/// returns true if the result is equal to the identity element.
///
/// Note: If the input is empty, this function returns true.
/// This is different to EIP2537 which disallows the empty input.
// revm-precompile-32.1.0/src/bn254/arkworks.rs:218-240 (renamed pairing_check → arkworks_pairing_check to avoid conflict with public wrapper)
#[inline]
fn arkworks_pairing_check(pairs: &[(&[u8], &[u8])]) -> Result<bool, PrecompileError> {
    let mut g1_points = Vec::with_capacity(pairs.len());
    let mut g2_points = Vec::with_capacity(pairs.len());

    for (g1_bytes, g2_bytes) in pairs {
        let g1 = read_g1_point(g1_bytes)?;
        let g2 = read_g2_point(g2_bytes)?;

        // Skip pairs where either point is at infinity
        if !g1.is_zero() && !g2.is_zero() {
            g1_points.push(g1);
            g2_points.push(g2);
        }
    }

    if g1_points.is_empty() {
        return Ok(true);
    }

    let pairing_result = Bn254::multi_pairing(&g1_points, &g2_points);
    Ok(pairing_result.0.is_one())
}

// ============================================================
// Public wrappers adapting the fixed-size byte-array API used by zkvm_accelerators.rs
//
// zkvm_accelerators.rs passes:
//   - g1_add:  &[u8; 64] for each G1 point
//   - g1_mul:  &[u8; 64] point, &[u8; 32] scalar
//   - pairing_check: &[[u8; 192]] slice of (64-byte G1 || 128-byte G2) pairs
// ============================================================

/// G1 point addition: two flat 64-byte points → 64-byte result.
pub fn g1_add(p1: &[u8; 64], p2: &[u8; 64]) -> Option<[u8; 64]> {
    g1_point_add(p1.as_ref(), p2.as_ref()).ok()
}

/// G1 scalar multiplication: 64-byte point + 32-byte scalar → 64-byte result.
pub fn g1_mul(point: &[u8; 64], scalar: &[u8; 32]) -> Option<[u8; 64]> {
    g1_point_mul(point.as_ref(), scalar.as_ref()).ok()
}

/// Pairing check: slice of (64-byte G1 || 128-byte G2) pairs → bool result.
pub fn pairing_check(pairs: &[[u8; 192]]) -> Option<bool> {
    let pair_slices: Vec<(&[u8], &[u8])> =
        pairs.iter().map(|pair| (&pair[..G1_LEN], &pair[G1_LEN..])).collect();
    arkworks_pairing_check(&pair_slices).ok()
}
