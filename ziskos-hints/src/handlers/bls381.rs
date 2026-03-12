use crate::{handlers::validate_hint_length, hint_fields, zisklib};

use anyhow::Result;

/// Processes an `HINT_BLS12_381_G1_ADD` hint.
#[inline]
pub fn bls12_381_g1_add_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![A: 96, B: 96];

    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 8) };

    validate_hint_length(bytes, EXPECTED_LEN, "HINT_BLS12_381_G1_ADD")?;

    let a: &[u8; A_SIZE] = bytes[A_OFFSET..A_OFFSET + A_SIZE].try_into().unwrap();
    let b: &[u8; B_SIZE] = bytes[B_OFFSET..B_OFFSET + B_SIZE].try_into().unwrap();

    let mut hints = Vec::new();
    let result: &mut [u8; 96] = &mut [0u8; 96];
    unsafe {
        zisklib::bls12_381_g1_add_c(result.as_mut_ptr(), a.as_ptr(), b.as_ptr(), &mut hints);
    }

    Ok(hints)
}

/// Processes an `HINT_BLS12_381_G1_MSM` hint.
#[inline]
pub fn bls12_381_g1_msm_hint(data: &[u64]) -> Result<Vec<u64>> {
    if data.is_empty() {
        anyhow::bail!("HINT_BLS12_381_G1_MSM: data is empty");
    }

    let num_pairs = data[0] as usize;

    const POINT_SIZE: usize = 96;
    const SCALAR_SIZE: usize = 32;
    const PAIR_SIZE_BYTES: usize = POINT_SIZE + SCALAR_SIZE;
    const PAIR_SIZE: usize = PAIR_SIZE_BYTES.div_ceil(8);

    let expected_len = 1 + num_pairs * PAIR_SIZE;

    validate_hint_length(data, expected_len, "HINT_BLS12_381_G1_MSM")?;

    let bytes = unsafe {
        std::slice::from_raw_parts(data.as_ptr().add(1) as *const u8, num_pairs * PAIR_SIZE_BYTES)
    };

    let mut hints = Vec::new();
    let result: &mut [u8; 96] = &mut [0u8; 96];
    unsafe {
        zisklib::bls12_381_g1_msm_c(result.as_mut_ptr(), bytes.as_ptr(), num_pairs, &mut hints);
    }

    Ok(hints)
}

/// Processes an `HINT_BLS12_381_G2_ADD` hint.
#[inline]
pub fn bls12_381_g2_add_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![A: 192, B: 192];

    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 8) };

    validate_hint_length(bytes, EXPECTED_LEN, "HINT_BLS12_381_G2_ADD")?;

    let a: &[u8; A_SIZE] = bytes[A_OFFSET..A_OFFSET + A_SIZE].try_into().unwrap();
    let b: &[u8; B_SIZE] = bytes[B_OFFSET..B_OFFSET + B_SIZE].try_into().unwrap();

    let mut hints = Vec::new();
    let result: &mut [u8; 192] = &mut [0u8; 192];
    unsafe {
        zisklib::bls12_381_g2_add_c(result.as_mut_ptr(), a.as_ptr(), b.as_ptr(), &mut hints);
    }

    Ok(hints)
}

/// Processes an `HINT_BLS12_381_G2_MSM` hint.
#[inline]
pub fn bls12_381_g2_msm_hint(data: &[u64]) -> Result<Vec<u64>> {
    if data.is_empty() {
        anyhow::bail!("HINT_BLS12_381_G1_MSM: data is empty");
    }

    let num_pairs = data[0] as usize;

    const POINT_SIZE: usize = 192;
    const SCALAR_SIZE: usize = 32;
    const PAIR_SIZE_BYTES: usize = POINT_SIZE + SCALAR_SIZE;
    const PAIR_SIZE: usize = PAIR_SIZE_BYTES.div_ceil(8);

    let expected_len = 1 + num_pairs * PAIR_SIZE;

    validate_hint_length(data, expected_len, "HINT_BLS12_381_G1_MSM")?;

    let bytes = unsafe {
        std::slice::from_raw_parts(data.as_ptr().add(1) as *const u8, num_pairs * PAIR_SIZE_BYTES)
    };

    let mut hints = Vec::new();
    let result: &mut [u8; 192] = &mut [0u8; 192];
    unsafe {
        zisklib::bls12_381_g2_msm_c(result.as_mut_ptr(), bytes.as_ptr(), num_pairs, &mut hints);
    }

    Ok(hints)
}

/// Processes an `HINT_BLS12_381_PAIRING_CHECK` hint.
#[inline]
pub fn bls12_381_pairing_check_hint(data: &[u64]) -> Result<Vec<u64>> {
    if data.is_empty() {
        anyhow::bail!("HINT_BLS12_381_G1_MSM: data is empty");
    }

    let num_pairs = data[0] as usize;

    const G1_SIZE: usize = 96;
    const G2_SIZE: usize = 192;
    const PAIR_SIZE_BYTES: usize = G1_SIZE + G2_SIZE;
    const PAIR_SIZE: usize = PAIR_SIZE_BYTES.div_ceil(8);

    let expected_len = 1 + num_pairs * PAIR_SIZE;

    validate_hint_length(data, expected_len, "HINT_BLS12_381_PAIRING_CHECK")?;

    let pairs = unsafe {
        std::slice::from_raw_parts(data.as_ptr().add(1) as *const u8, num_pairs * PAIR_SIZE_BYTES)
    };

    let mut hints = Vec::new();
    unsafe {
        zisklib::bls12_381_pairing_check_c(pairs.as_ptr(), num_pairs, &mut hints);
    }

    Ok(hints)
}

/// Processes an `HINT_BLS12_381_FP_TO_G1` hint.
#[inline]
pub fn bls12_381_fp_to_g1_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![FP: 6];

    validate_hint_length(data, EXPECTED_LEN, "HINT_BLS12_381_FP_TO_G1")?;

    let fp: &[u64; FP_SIZE] = data[FP_OFFSET..FP_OFFSET + FP_SIZE].try_into().unwrap();

    let mut hints = Vec::new();
    let result: &mut [u8; 96] = &mut [0u8; 96];
    unsafe {
        zisklib::bls12_381_fp_to_g1_c(result.as_mut_ptr(), fp.as_ptr() as *const u8, &mut hints);
    }

    Ok(hints)
}

/// Processes an `HINT_BLS12_381_FP2_TO_G2` hint.
#[inline]
pub fn bls12_381_fp2_to_g2_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![FP2: 12];

    validate_hint_length(data, EXPECTED_LEN, "HINT_BLS12_381_FP2_TO_G2")?;

    let fp2: &[u64; FP2_SIZE] = data[FP2_OFFSET..FP2_OFFSET + FP2_SIZE].try_into().unwrap();

    let mut hints = Vec::new();
    let result: &mut [u8; 192] = &mut [0u8; 192];
    unsafe {
        zisklib::bls12_381_fp2_to_g2_c(result.as_mut_ptr(), fp2.as_ptr() as *const u8, &mut hints);
    }

    Ok(hints)
}
