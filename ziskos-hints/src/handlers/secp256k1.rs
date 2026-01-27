use crate::handlers::u64_be_to_u64_le;
use crate::handlers::validate_hint_length;
use crate::hint_fields;
use crate::zisklib;

use anyhow::Result;

/// Processes an `HINT_SECP256K1_ECRECOVER` hint.
#[inline]
pub fn secp256k1_ecrecover_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![R: 4, S: 4, RECID: 1, MSG: 4, LO_S: 1];

    validate_hint_length(data, EXPECTED_LEN, "HINT_SECP256K1_ECRECOVER")?;

    let r: &[u64; R_SIZE] = data[R_OFFSET..R_OFFSET + R_SIZE].try_into().unwrap();
    let s: &[u64; S_SIZE] = data[S_OFFSET..S_OFFSET + S_SIZE].try_into().unwrap();
    let recid = u64::from_le(data[RECID_OFFSET]) as u8;
    let msg: &[u64; MSG_SIZE] = data[MSG_OFFSET..MSG_OFFSET + MSG_SIZE].try_into().unwrap();
    let low_s: bool = u64::from_le(data[LO_S_OFFSET]) != 0;

    let mut hints = Vec::new();

    let r = u64_be_to_u64_le(r);
    let s = u64_be_to_u64_le(s);
    let msg = u64_be_to_u64_le(msg);

    zisklib::secp256k1_ecrecover_point(&r, &s, &msg, recid, low_s, &mut hints)
        .map_err(|e: u8| anyhow::anyhow!("HINT_SECP256K1_ECRECOVER:  {}", e))?;

    Ok(hints)
}
