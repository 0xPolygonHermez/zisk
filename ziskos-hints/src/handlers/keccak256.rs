use crate::{handlers::validate_hint_length, zisklib};

use anyhow::Result;

/// Processes an `HINT_KECCAK256` hint.
#[inline]
pub fn keccak256_hint(data: &[u64], data_len_bytes: usize) -> Result<Vec<u64>> {
    let data_len_words = data_len_bytes.div_ceil(8);

    if data.len() != data_len_words {
        anyhow::bail!(
            "HINT_KECCAK256: expected data length of {} bytes ({} words), got {} words",
            data_len_bytes,
            data_len_words,
            data.len()
        );
    }

    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data_len_bytes) };

    validate_hint_length(bytes, data_len_bytes, "HINT_KECCAK256")?;

    let mut hints = Vec::new();
    zisklib::keccak256(bytes, &mut hints);

    Ok(hints)
}
