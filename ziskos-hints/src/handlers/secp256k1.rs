use crate::handlers::validate_hint_length;
use crate::hint_fields;
use crate::zisklib;

use anyhow::Result;

/// Processes an `HINT_SECP256K1_ECDSA_ADDRESS_RECOVER` hint.
#[inline]
pub fn secp256k1_ecdsa_address_recover(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![SIG: 8, RECID: 1, MSG: 4];

    validate_hint_length(data, EXPECTED_LEN, "HINT_SECP256K1_ECDSA_ADDRESS_RECOVER")?;

    let sig: &[u64; SIG_SIZE] = data[SIG_OFFSET..SIG_OFFSET + SIG_SIZE].try_into().unwrap();
    let recid: u8 = data[RECID_OFFSET] as u8;
    let msg: &[u64; MSG_SIZE] = data[MSG_OFFSET..MSG_OFFSET + MSG_SIZE].try_into().unwrap();

    let mut hints = Vec::new();
    let result: &mut [u8; 32] = &mut [0u8; 32];
    unsafe {
        zisklib::secp256k1_ecdsa_address_recover_c(
            sig.as_ptr() as *const u8,
            recid,
            msg.as_ptr() as *const u8,
            result.as_mut_ptr(),
            &mut hints,
        );
    }

    Ok(hints)
}

/// Processes an `HINT_SECP256K1_ECDSA_VERIFY_ADDRESS_RECOVER` hint.
#[inline]
pub fn secp256k1_ecdsa_verify_address_recover(data: &[u64]) -> Result<Vec<u64>> {
    hint_fields![SIG: 8, MSG: 4, PK: 8];

    validate_hint_length(data, EXPECTED_LEN, "HINT_SECP256K1_ECDSA_VERIFY_ADDRESS_RECOVER")?;

    let sig: &[u64; SIG_SIZE] = data[SIG_OFFSET..SIG_OFFSET + SIG_SIZE].try_into().unwrap();
    let msg: &[u64; MSG_SIZE] = data[MSG_OFFSET..MSG_OFFSET + MSG_SIZE].try_into().unwrap();
    let pk: &[u64; PK_SIZE] = data[PK_OFFSET..PK_OFFSET + PK_SIZE].try_into().unwrap();

    let mut hints = Vec::new();
    let result: &mut [u8; 32] = &mut [0u8; 32];
    unsafe {
        zisklib::secp256k1_ecdsa_verify_and_address_recover_c(
            sig.as_ptr() as *const u8,
            msg.as_ptr() as *const u8,
            pk.as_ptr() as *const u8,
            result.as_mut_ptr(),
            &mut hints,
        );
    }

    Ok(hints)
}
