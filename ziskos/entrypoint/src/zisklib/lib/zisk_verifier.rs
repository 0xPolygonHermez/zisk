pub fn verify_zisk_proof(zisk_proof: &[u64]) -> bool {
    if zisk_proof.len() < zisk_verifier::VADCOP_VK_LEN_WORDS {
        return false;
    }
    let (proof, vk) = zisk_proof.split_at(zisk_proof.len() - zisk_verifier::VADCOP_VK_LEN_WORDS);
    zisk_verifier::verify_vadcop_final_proof(proof, vk)
}
