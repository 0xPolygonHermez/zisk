use anyhow::{bail, Result};

/// Host-side handler: permutes the eight packed words carried by the hint.
pub fn koala_poseidon2_hint(data: &[u64], data_len_bytes: usize) -> Result<Vec<u64>> {
    if data_len_bytes != 64 || data.len() != 8 {
        bail!("KoalaBear Poseidon2 requires exactly 64 bytes / eight packed words");
    }
    let mut words: [u64; 8] = data.try_into().expect("length checked");
    zisk_definitions::koala_poseidon2::permute_packed(&mut words)
        .map_err(|error| anyhow::anyhow!("noncanonical KoalaBear input lane {}", error.0))?;
    Ok(words.to_vec())
}
