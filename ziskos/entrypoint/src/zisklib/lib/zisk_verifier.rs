/// Hash family the recursion was proven with, fixed at guest build time by the
/// `stark-poseidon1` feature of this crate.
const HASH: &str = if cfg!(feature = "stark-poseidon1") { "Poseidon1" } else { "Poseidon2" };

pub fn verify_zisk_proof(zisk_proof: &[u64]) -> bool {
    if zisk_proof.len() < zisk_verifier::VADCOP_VK_LEN_WORDS {
        return false;
    }
    let (proof, vk) = zisk_proof.split_at(zisk_proof.len() - zisk_verifier::VADCOP_VK_LEN_WORDS);
    zisk_verifier::verify_vadcop_final_proof(proof, vk, HASH)
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
