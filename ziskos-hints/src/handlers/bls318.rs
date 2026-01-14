use crate::{handlers::validate_hint_length, hint_fields, zisklib};

/// Processes an MUL_FP12_BLS12_381 hint.
#[inline]
pub fn mul_fp12_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![A: 72, B: 72];

    validate_hint_length(data, EXPECTED_LEN, "MUL_FP12_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let a: &[u64; A_SIZE] = data[A_OFFSET..A_OFFSET + A_SIZE].try_into().unwrap();
    let b: &[u64; B_SIZE] = data[B_OFFSET..B_OFFSET + B_SIZE].try_into().unwrap();

    let mut processed_hints = Vec::new();

    zisklib::mul_fp12_bls12_381(a, b, &mut processed_hints);

    Ok(processed_hints)
}

/// Processes a DECOMPRESS_BLS12_381 hint.
#[inline]
pub fn decompress_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![INPUT: 6];

    validate_hint_length(data, EXPECTED_LEN, "DECOMPRESS_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let input: &[u64; INPUT_SIZE] =
        data[INPUT_OFFSET..INPUT_OFFSET + INPUT_SIZE].try_into().unwrap();
    // Map a [u64; 6] to a [u8; 48]
    let input: &[u8; INPUT_SIZE * 8] = unsafe { &*(input.as_ptr() as *const [u8; INPUT_SIZE * 8]) };

    let mut processed_hints = Vec::new();

    zisklib::decompress_bls12_381(input, &mut processed_hints)?;

    Ok(processed_hints)
}

/// Processes an IS_ON_CURVE_BLS12_381 hint.
#[inline]
pub fn is_on_curve_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![P: 12];

    validate_hint_length(data, EXPECTED_LEN, "IS_ON_CURVE_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let p: &[u64; P_SIZE] = data[P_OFFSET..P_OFFSET + P_SIZE].try_into().unwrap();

    let mut processed_hints = Vec::new();

    zisklib::is_on_curve_bls12_381(p, &mut processed_hints);

    Ok(processed_hints)
}

/// Processes an IS_ON_SUBGROUP_BLS12_381 hint.
#[inline]
pub fn is_on_subgroup_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![P: 12];

    validate_hint_length(data, EXPECTED_LEN, "IS_ON_SUBGROUP_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let p: &[u64; P_SIZE] = data[P_OFFSET..P_OFFSET + P_SIZE].try_into().unwrap();

    let mut processed_hints = Vec::new();

    zisklib::is_on_subgroup_bls12_381(p, &mut processed_hints);

    Ok(processed_hints)
}

/// Processes an ADD_BLS12_381 hint.
#[inline]
pub fn add_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![P1: 12, P2: 12];

    validate_hint_length(data, EXPECTED_LEN, "ADD_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let p1: &[u64; P1_SIZE] = data[P1_OFFSET..P1_OFFSET + P1_SIZE].try_into().unwrap();
    let p2: &[u64; P2_SIZE] = data[P2_OFFSET..P2_OFFSET + P2_SIZE].try_into().unwrap();

    let mut processed_hints = Vec::new();

    zisklib::add_bls12_381(p1, p2, &mut processed_hints);

    Ok(processed_hints)
}

/// Processes a SCALAR_MUL_BLS12_381 hint.
#[inline]
pub fn scalar_mul_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![P: 12, K: 6];

    validate_hint_length(data, EXPECTED_LEN, "SCALAR_MUL_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let p: &[u64; P_SIZE] = data[P_OFFSET..P_OFFSET + P_SIZE].try_into().unwrap();
    let k: &[u64; K_SIZE] = data[K_OFFSET..K_OFFSET + K_SIZE].try_into().unwrap();

    let mut processed_hints = Vec::new();

    zisklib::scalar_mul_bls12_381(p, k, &mut processed_hints);

    Ok(processed_hints)
}

/// Processes a DECOMPRESS_TWIST_BLS12_381 hint.
#[inline]
pub fn decompress_twist_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![INPUT: 12];

    validate_hint_length(data, EXPECTED_LEN, "DECOMPRESS_TWIST_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let input: &[u64; INPUT_SIZE] =
        data[INPUT_OFFSET..INPUT_OFFSET + INPUT_SIZE].try_into().unwrap();
    // Map a [u64; 6] to a [u8; 48]
    let input: &[u8; INPUT_SIZE * 8] = unsafe { &*(input.as_ptr() as *const [u8; INPUT_SIZE * 8]) };

    let mut processed_hints = Vec::new();

    zisklib::decompress_twist_bls12_381(input, &mut processed_hints)?;

    Ok(processed_hints)
}

/// Processes an IS_ON_CURVE_TWIST_BLS12_381 hint.
#[inline]
pub fn is_on_curve_twist_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![P: 24];

    validate_hint_length(data, EXPECTED_LEN, "IS_ON_CURVE_TWIST_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let p: &[u64; P_SIZE] = data[P_OFFSET..P_OFFSET + P_SIZE].try_into().unwrap();

    let mut processed_hints = Vec::new();

    zisklib::is_on_curve_twist_bls12_381(p, &mut processed_hints);

    Ok(processed_hints)
}

/// Processes an IS_ON_SUBGROUP_TWIST_BLS12_381 hint.
#[inline]
pub fn is_on_subgroup_twist_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![P: 24];

    validate_hint_length(data, EXPECTED_LEN, "IS_ON_SUBGROUP_TWIST_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let p: &[u64; P_SIZE] = data[P_OFFSET..P_OFFSET + P_SIZE].try_into().unwrap();

    let mut processed_hints = Vec::new();

    zisklib::is_on_subgroup_twist_bls12_381(p, &mut processed_hints);

    Ok(processed_hints)
}

/// Processes an ADD_TWIST_BLS12_381 hint.
#[inline]
pub fn add_twist_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![P1: 24, P2: 24];

    validate_hint_length(data, EXPECTED_LEN, "ADD_TWIST_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let p1: &[u64; P1_SIZE] = data[P1_OFFSET..P1_OFFSET + P1_SIZE].try_into().unwrap();
    let p2: &[u64; P2_SIZE] = data[P2_OFFSET..P2_OFFSET + P2_SIZE].try_into().unwrap();

    let mut processed_hints = Vec::new();

    zisklib::add_twist_bls12_381(p1, p2, &mut processed_hints);

    Ok(processed_hints)
}

/// Processes a SCALAR_MUL_TWIST_BLS12_381 hint.
#[inline]
pub fn scalar_mul_twist_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![P: 24, K: 6];

    validate_hint_length(data, EXPECTED_LEN, "SCALAR_MUL_TWIST_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let p: &[u64; P_SIZE] = data[P_OFFSET..P_OFFSET + P_SIZE].try_into().unwrap();
    let k: &[u64; K_SIZE] = data[K_OFFSET..K_OFFSET + K_SIZE].try_into().unwrap();

    let mut processed_hints = Vec::new();

    zisklib::scalar_mul_twist_bls12_381(p, k, &mut processed_hints);

    Ok(processed_hints)
}

/// Processes a MILLER_LOOP_BLS12_381 hint.
#[inline]
pub fn miller_loop_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![P: 12, Q: 24];

    validate_hint_length(data, EXPECTED_LEN, "MILLER_LOOP_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let p: &[u64; P_SIZE] = data[P_OFFSET..P_OFFSET + P_SIZE].try_into().unwrap();
    let q: &[u64; Q_SIZE] = data[Q_OFFSET..Q_OFFSET + Q_SIZE].try_into().unwrap();

    let mut processed_hints = Vec::new();

    zisklib::miller_loop_bls12_381(p, q, &mut processed_hints);

    Ok(processed_hints)
}

/// Processes a FINAL_EXP_BLS12_381 hint.
#[inline]
pub fn final_exp_bls12_381_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![F: 72];

    validate_hint_length(data, EXPECTED_LEN, "FINAL_EXP_BLS12_381")?;

    // Safe to unwrap due to prior length validation.
    let f: &[u64; F_SIZE] = data[F_OFFSET..F_OFFSET + F_SIZE].try_into().unwrap();

    let mut processed_hints = Vec::new();

    zisklib::final_exp_bls12_381(f, &mut processed_hints);

    Ok(processed_hints)
}
