//! secp256k1 ECDSA software fallbacks using the secp256k1 crate (non-hints, non-zkVM builds only).
//! Source: revm-precompile-32.1.0/src/secp256k1/bitcoin_secp256k1.rs

/// Verifies a secp256k1 ECDSA signature. Returns true if valid.
pub fn verify(sig_bytes: &[u8; 64], msg_bytes: [u8; 32], pk_bytes: &[u8; 64]) -> bool {
    use secp256k1::{ecdsa::Signature, Message, PublicKey};
    (|| -> Option<()> {
        let signature = Signature::from_compact(sig_bytes).ok()?;
        let mut full_pk = [0u8; 65];
        full_pk[0] = 4;
        full_pk[1..].copy_from_slice(pk_bytes);
        let public_key = PublicKey::from_slice(&full_pk).ok()?;
        let message = Message::from_digest(msg_bytes);
        let secp = secp256k1::Secp256k1::verification_only();
        secp.verify_ecdsa(message, &signature, &public_key).ok()
    })()
    .is_some()
}

/// Recovers the uncompressed public key (64 bytes, without the 0x04 prefix) from a recoverable
/// ECDSA signature. Returns `None` on failure.
pub fn ecrecover(sig_bytes: &[u8; 64], recid: u8, msg_bytes: [u8; 32]) -> Option<[u8; 64]> {
    use secp256k1::{
        ecdsa::{RecoverableSignature, RecoveryId},
        Message,
    };
    let recovery_id = RecoveryId::try_from(recid as i32).ok()?;
    let recoverable_sig = RecoverableSignature::from_compact(sig_bytes, recovery_id).ok()?;
    let message = Message::from_digest(msg_bytes);
    let secp = secp256k1::Secp256k1::new();
    let public_key = secp.recover_ecdsa(message, &recoverable_sig).ok()?;
    let pk_bytes = public_key.serialize_uncompressed(); // 65 bytes: 0x04 || x || y
    let mut out = [0u8; 64];
    out.copy_from_slice(&pk_bytes[1..]);
    Some(out)
}
