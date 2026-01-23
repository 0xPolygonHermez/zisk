use crate::{handlers::read_field_bytes, zisklib};

use anyhow::Result;

// Processes a `MODEXP` hint.
#[inline]
pub fn modexp_hint(data: &[u64]) -> Result<Vec<u64>> {
    let mut pos = 0;
    let (base, base_len) = read_field_bytes(data, &mut pos)?;
    let (exp, exp_len) = read_field_bytes(data, &mut pos)?;
    let (modulus, modulus_len) = read_field_bytes(data, &mut pos)?;

    // validate_hint_length(data, pos, "MODEXP")?;

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
