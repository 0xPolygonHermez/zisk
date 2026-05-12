use proofman_verifier::{verify_vadcop_final_u64, verify_vadcop_final_compressed_u64};

pub fn verify_vadcop_final_proof(zisk_proof: &[u64], vadcop_final_vk: &[u64]) -> bool {
    // Format: [minimal(1)][n_publics(1)][publics][proof]

    if zisk_proof.is_empty() {
        return false;
    }

    let minimal = zisk_proof[0] == 1;
    let vadcop_proof = &zisk_proof[1..];

    if minimal {
        verify_vadcop_final_compressed_u64(vadcop_proof, vadcop_final_vk)
    } else {
        verify_vadcop_final_u64(vadcop_proof, vadcop_final_vk)
    }
}
