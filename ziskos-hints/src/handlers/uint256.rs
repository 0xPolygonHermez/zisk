use crate::handlers::validate_hint_length;
use crate::hint_fields;
use crate::zisklib;

use anyhow::Result;

#[inline]
pub fn mulmod256_hint(data: &[u64]) -> Result<Vec<u64>> {
    // a, b, m: each 32 bytes = 4 u64 words (big-endian operands).
    hint_fields![A: 4, B: 4, M: 4];

    validate_hint_length(data, EXPECTED_LEN, "HINT_MULMOD256")?;

    let a: &[u64; A_SIZE] = data[A_OFFSET..A_OFFSET + A_SIZE].try_into().unwrap();
    let b: &[u64; B_SIZE] = data[B_OFFSET..B_OFFSET + B_SIZE].try_into().unwrap();
    let m: &[u64; M_SIZE] = data[M_OFFSET..M_OFFSET + M_SIZE].try_into().unwrap();

    let mut hints = Vec::new();
    let result: &mut [u8; 32] = &mut [0u8; 32];
    unsafe {
        zisklib::mul_mod_bytes256_c(
            a.as_ptr() as *const u8,
            b.as_ptr() as *const u8,
            m.as_ptr() as *const u8,
            result.as_mut_ptr(),
            &mut hints,
        );
    }

    Ok(hints)
}

/// `a mod m` — recompute the witness from the recorded 32-byte big-endian operands.
#[inline]
pub fn reduce_mod256_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![A: 4, M: 4];
    validate_hint_length(data, EXPECTED_LEN, "HINT_REDUCE_MOD256")?;
    let a: &[u64; A_SIZE] = data[A_OFFSET..A_OFFSET + A_SIZE].try_into().unwrap();
    let m: &[u64; M_SIZE] = data[M_OFFSET..M_OFFSET + M_SIZE].try_into().unwrap();
    let mut hints = Vec::new();
    let result: &mut [u8; 32] = &mut [0u8; 32];
    unsafe {
        zisklib::reduce_mod_bytes256_c(
            a.as_ptr() as *const u8,
            m.as_ptr() as *const u8,
            result.as_mut_ptr(),
            &mut hints,
        );
    }
    Ok(hints)
}

/// `(a + b) mod m`.
#[inline]
pub fn add_mod256_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![A: 4, B: 4, M: 4];
    validate_hint_length(data, EXPECTED_LEN, "HINT_ADD_MOD256")?;
    let a: &[u64; A_SIZE] = data[A_OFFSET..A_OFFSET + A_SIZE].try_into().unwrap();
    let b: &[u64; B_SIZE] = data[B_OFFSET..B_OFFSET + B_SIZE].try_into().unwrap();
    let m: &[u64; M_SIZE] = data[M_OFFSET..M_OFFSET + M_SIZE].try_into().unwrap();
    let mut hints = Vec::new();
    let result: &mut [u8; 32] = &mut [0u8; 32];
    unsafe {
        zisklib::add_mod_bytes256_c(
            a.as_ptr() as *const u8,
            b.as_ptr() as *const u8,
            m.as_ptr() as *const u8,
            result.as_mut_ptr(),
            &mut hints,
        );
    }
    Ok(hints)
}

/// `a² mod m`.
#[inline]
pub fn square_mod256_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![A: 4, M: 4];
    validate_hint_length(data, EXPECTED_LEN, "HINT_SQUARE_MOD256")?;
    let a: &[u64; A_SIZE] = data[A_OFFSET..A_OFFSET + A_SIZE].try_into().unwrap();
    let m: &[u64; M_SIZE] = data[M_OFFSET..M_OFFSET + M_SIZE].try_into().unwrap();
    let mut hints = Vec::new();
    let result: &mut [u8; 32] = &mut [0u8; 32];
    unsafe {
        zisklib::square_mod_bytes256_c(
            a.as_ptr() as *const u8,
            m.as_ptr() as *const u8,
            result.as_mut_ptr(),
            &mut hints,
        );
    }
    Ok(hints)
}

/// `base^exp mod m`.
#[inline]
pub fn pow_mod256_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![BASE: 4, EXP: 4, M: 4];
    validate_hint_length(data, EXPECTED_LEN, "HINT_POW_MOD256")?;
    let base: &[u64; BASE_SIZE] = data[BASE_OFFSET..BASE_OFFSET + BASE_SIZE].try_into().unwrap();
    let exp: &[u64; EXP_SIZE] = data[EXP_OFFSET..EXP_OFFSET + EXP_SIZE].try_into().unwrap();
    let m: &[u64; M_SIZE] = data[M_OFFSET..M_OFFSET + M_SIZE].try_into().unwrap();
    let mut hints = Vec::new();
    let result: &mut [u8; 32] = &mut [0u8; 32];
    unsafe {
        zisklib::pow_mod_bytes256_c(
            base.as_ptr() as *const u8,
            exp.as_ptr() as *const u8,
            m.as_ptr() as *const u8,
            result.as_mut_ptr(),
            &mut hints,
        );
    }
    Ok(hints)
}

/// `a⁻¹ mod m` (the C return status is ignored; only the witness matters).
#[inline]
pub fn inv_mod256_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![A: 4, M: 4];
    validate_hint_length(data, EXPECTED_LEN, "HINT_INV_MOD256")?;
    let a: &[u64; A_SIZE] = data[A_OFFSET..A_OFFSET + A_SIZE].try_into().unwrap();
    let m: &[u64; M_SIZE] = data[M_OFFSET..M_OFFSET + M_SIZE].try_into().unwrap();
    let mut hints = Vec::new();
    let result: &mut [u8; 32] = &mut [0u8; 32];
    unsafe {
        let _ = zisklib::inv_mod_bytes256_c(
            a.as_ptr() as *const u8,
            m.as_ptr() as *const u8,
            result.as_mut_ptr(),
            &mut hints,
        );
    }
    Ok(hints)
}
