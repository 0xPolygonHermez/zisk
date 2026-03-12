use crate::handlers::validate_hint_length;
use crate::hint_fields;
use crate::zisklib;

use anyhow::Result;

/// Processes an `HINT_SECP256R1_ECDSA_VERIFY` hint.
#[inline]
pub fn secp256r1_ecdsa_verify_hint(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![MSG: 4, SIG: 8, PK: 8];

    validate_hint_length(data, EXPECTED_LEN, "HINT_SECP256R1_ECDSA_VERIFY")?;

    let msg: &[u64; MSG_SIZE] = data[MSG_OFFSET..MSG_OFFSET + MSG_SIZE].try_into().unwrap();
    let sig: &[u64; SIG_SIZE] = data[SIG_OFFSET..SIG_OFFSET + SIG_SIZE].try_into().unwrap();
    let pk: &[u64; PK_SIZE] = data[PK_OFFSET..PK_OFFSET + PK_SIZE].try_into().unwrap();

    let mut hints = Vec::new();
    unsafe {
        zisklib::secp256r1_ecdsa_verify_c(
            msg.as_ptr() as *const u8,
            sig.as_ptr() as *const u8,
            pk.as_ptr() as *const u8,
            &mut hints,
        );
    }

    Ok(hints)
}
