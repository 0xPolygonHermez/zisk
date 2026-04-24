//! RIPEMD-160 software fallback using ripemd crate (non-hints, non-zkVM builds only).
//! Returns 32 bytes: 12 zero-padding bytes followed by the 20-byte digest.
pub fn hash(data: &[u8]) -> [u8; 32] {
    use ripemd::Digest;
    let digest = ripemd::Ripemd160::digest(data);
    let mut out = [0u8; 32];
    out[12..].copy_from_slice(&digest);
    out
}
