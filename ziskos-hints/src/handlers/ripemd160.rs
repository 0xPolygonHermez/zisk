use crate::zisklib;

use anyhow::Result;

/// Processes a `HINT_RIPEMD160` hint.
#[inline]
pub fn ripemd160_hint(data: &[u64], data_len_bytes: usize) -> Result<Vec<u64>> {
    let data_len_words = data_len_bytes.div_ceil(8);

    if data.len() != data_len_words {
        anyhow::bail!(
            "HINT_RIPEMD160: expected data length of {} bytes ({} words), got {} words",
            data_len_bytes,
            data_len_words,
            data.len()
        );
    }

    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data_len_bytes) };

    let mut hints = Vec::new();
    zisklib::ripemd160(bytes, &mut hints);

    Ok(hints)
}
