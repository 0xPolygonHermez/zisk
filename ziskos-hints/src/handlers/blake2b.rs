use crate::{handlers::validate_hint_length, hint_fields, zisklib};

use anyhow::Result;

/// Processes an `HINT_BLAKE2B_COMPRESS` hint.
#[inline]
pub fn blake2b_compress_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![ROUNDS: 1, STATE: 8, MESSAGE: 16, OFFSET: 2, FINAL_BLOCK: 1];

    validate_hint_length(data, EXPECTED_LEN, "HINT_BLAKE2B_COMPRESS")?;

    let rounds = data[ROUNDS_OFFSET] as u32;
    let mut state: [u64; 8] = data[STATE_OFFSET..STATE_OFFSET + STATE_SIZE].try_into().unwrap();
    let message = data[MESSAGE_OFFSET..MESSAGE_OFFSET + MESSAGE_SIZE].try_into().unwrap();
    let offset = data[OFFSET_OFFSET..OFFSET_OFFSET + OFFSET_SIZE].try_into().unwrap();
    let final_block = data[FINAL_BLOCK_OFFSET] != 0;

    let mut hints = Vec::new();
    zisklib::blake2b_compress(rounds, &mut state, message, offset, final_block, &mut hints);

    Ok(hints)
}
