use crate::{handlers::read_field_bytes, zisklib};

use anyhow::Result;

// Processes a `MODEXP` hint.
#[inline]
pub fn modexp_hint(data: &[u64]) -> Result<Vec<u64>> {
    let mut pos = 0;
    let (base, base_len) = read_field_bytes(data, &mut pos)?;
    let (exp, exp_len) = read_field_bytes(data, &mut pos)?;
    let (modulus, modulus_len) = read_field_bytes(data, &mut pos)?;

    // Check that the data length fits the expected length.
    // Each length prefix is 8 bytes (1 u64), so total prefix size is 3 * 8 = 24 bytes.
    // The total expected length in bytes is 24 + base_len + exp_len + modulus_len.
    if (24 + base_len + exp_len + modulus_len).div_ceil(8) > data.len() * 8 {
        anyhow::bail!("MODEXP hint data too short");
    }

    let mut hints = Vec::new();
    let mut result = vec![0u8; modulus_len];
    unsafe {
        zisklib::modexp_bytes_c(
            base.as_ptr(),
            base_len,
            exp.as_ptr(),
            exp_len,
            modulus.as_ptr(),
            modulus_len,
            result.as_mut_ptr(),
            &mut hints,
        );
    }

    Ok(hints)
}
