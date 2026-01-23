use crate::{handlers::validate_hint_min_length, zisklib};

use anyhow::Result;

/// Processes an `HINT_SHA256` hint.
#[inline]
pub fn sha256_hint(data: &[u64], data_len_bytes: usize) -> Result<Vec<u64>> {
    let data_len_u64 = data_len_bytes.div_ceil(8);

    validate_hint_min_length(data, data_len_u64, "HINT_SHA256")?;

    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data_len_bytes) };

    let mut hints = Vec::new();
    zisklib::sha256(bytes, &mut hints);

    Ok(hints)
}
