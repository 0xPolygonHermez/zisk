use crate::handlers::validate_hint_length;
use crate::hint_fields;
use crate::zisklib;

/// Processes a REDMOD256 hint.
#[inline]
pub fn redmod256_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![A: 4, M: 4];

    validate_hint_length(data, EXPECTED_LEN, "REDMOD256")?;

    let mut result: [u64; 4] = [0; 4];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::redmod256_c(
            &data[A_OFFSET],
            &data[M_OFFSET],
            &mut result[0],
            &mut processed_hints,
        );
    }

    Ok(processed_hints)
}

/// Processes an ADDMOD256 hint.
#[inline]
pub fn addmod256_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![A: 4, B: 4, M: 4];

    validate_hint_length(data, EXPECTED_LEN, "ADDMOD256")?;

    let mut result: [u64; 4] = [0; 4];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::addmod256_c(
            &data[A_OFFSET],
            &data[B_OFFSET],
            &data[M_OFFSET],
            &mut result[0],
            &mut processed_hints,
        );
    }

    Ok(processed_hints)
}

/// Processes a MULMOD256 hint.
#[inline]
pub fn mulmod256_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![A: 4, B: 4, M: 4];

    validate_hint_length(data, EXPECTED_LEN, "MULMOD256")?;

    let mut result: [u64; 4] = [0; 4];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::mulmod256_c(
            &data[A_OFFSET],
            &data[B_OFFSET],
            &data[M_OFFSET],
            &mut result[0],
            &mut processed_hints,
        );
    }

    Ok(processed_hints)
}

/// Processes a DIVREM256 hint.
#[inline]
pub fn divrem256_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![A: 4, B: 4];

    validate_hint_length(data, EXPECTED_LEN, "DIVREM256")?;

    let mut processed_hints = Vec::new();

    let mut q: [u64; 4] = [0; 4];
    let mut r: [u64; 4] = [0; 4];

    unsafe {
        zisklib::divrem256_c(
            &data[A_OFFSET],
            &data[B_OFFSET],
            &mut q[0],
            &mut r[0],
            &mut processed_hints,
        );
    }

    Ok(processed_hints)
}

/// Processes a WPOW256 hint.
#[inline]
pub fn wpow256_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![A: 4, EXP: 4];

    validate_hint_length(data, EXPECTED_LEN, "WPOW256")?;

    let mut result: [u64; 4] = [0; 4];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::wpow256_c(
            &data[A_OFFSET],
            &data[EXP_OFFSET],
            &mut result[0],
            &mut processed_hints,
        );
    }

    Ok(processed_hints)
}

/// Processes an OMUL256 hint.
#[inline]
pub fn omul256_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![A: 4, B: 4];

    validate_hint_length(data, EXPECTED_LEN, "OMUL256")?;

    let mut result: [u64; 4] = [0; 4];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::omul256_c(&data[A_OFFSET], &data[B_OFFSET], &mut result[0], &mut processed_hints);
    }

    Ok(processed_hints)
}

/// Processes a WMUL256 hint.
#[inline]
pub fn wmul256_hint(data: &[u64]) -> Result<Vec<u64>, String> {
    hint_fields![A: 4, B: 4];

    validate_hint_length(data, EXPECTED_LEN, "WMUL256")?;

    let mut result: [u64; 4] = [0; 4];
    let mut processed_hints = Vec::new();

    unsafe {
        zisklib::wmul256_c(&data[A_OFFSET], &data[B_OFFSET], &mut result[0], &mut processed_hints);
    }

    Ok(processed_hints)
}
