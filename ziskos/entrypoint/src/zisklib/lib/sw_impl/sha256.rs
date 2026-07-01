//! SHA-256 software fallback using sha2 crate (non-hints, non-zkVM builds only).
pub fn hash(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}
