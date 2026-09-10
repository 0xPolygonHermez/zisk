/// Hash families a proof can be produced under. Checked before dispatch: the verifier
/// panics on an unknown family, and a guest panic aborts the run.
const HASH_FAMILIES: [&str; 3] = ["Poseidon1", "Poseidon2", "blake3"];

/// Family assumed when the caller names none. A proof carries no family tag, so proofs
/// from a Poseidon proving key must go through [verify_zisk_proof_with_hash].
const DEFAULT_HASH: &str = "blake3";

pub fn verify_zisk_proof(zisk_proof: &[u64]) -> bool {
    verify_zisk_proof_with_hash(zisk_proof, DEFAULT_HASH)
}

/// Verify a proof produced under `hash`, the hash family of the proving key the recursion
/// was generated against. Returns `false` for an unrecognized family.
pub fn verify_zisk_proof_with_hash(zisk_proof: &[u64], hash: &str) -> bool {
    if !HASH_FAMILIES.contains(&hash) {
        return false;
    }
    if zisk_proof.len() < zisk_verifier::VADCOP_VK_LEN_WORDS {
        return false;
    }
    let (proof, vk) = zisk_proof.split_at(zisk_proof.len() - zisk_verifier::VADCOP_VK_LEN_WORDS);
    zisk_verifier::verify_vadcop_final_proof(proof, vk, hash)
}

/// C-ABI wrapper around [verify_zisk_proof] for C/C++ call sites.
///
/// # Safety
/// - `zisk_proof` must point to at least `zisk_proof_len` valid, initialized bytes
/// - `zisk_proof` must be 8-byte aligned and `zisk_proof_len` a multiple of 8;
///   the function returns `false` otherwise.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_verify_zisk_proof_c")]
pub unsafe extern "C" fn verify_zisk_proof_c(zisk_proof: *const u8, zisk_proof_len: usize) -> bool {
    let zisk_proof_bytes = core::slice::from_raw_parts(zisk_proof, zisk_proof_len);
    let (prefix, words, suffix) = zisk_proof_bytes.align_to::<u64>();
    if !prefix.is_empty() || !suffix.is_empty() {
        return false;
    }
    verify_zisk_proof(words)
}
