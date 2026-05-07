use proofman_verifier::{verify_vadcop_final_bytes, verify_vadcop_final_compressed_bytes};

pub fn verify_vadcop_final_proof(zisk_proof: &[u8], vadcop_final_vk: &[u8]) -> bool {
    // Format: [compressed(8)][pubs_len(8)][pubs][proof_bytes]

    if zisk_proof.len() < 8 {
        return false;
    }

    // Read minimal flag (8 bytes, u64 little-endian)
    let minimal = u64::from_le_bytes(zisk_proof[..8].try_into().unwrap()) == 1;
    let vadcop_proof = &zisk_proof[8..];

    if minimal {
        verify_vadcop_final_compressed_bytes(vadcop_proof, vadcop_final_vk)
    } else {
        verify_vadcop_final_bytes(vadcop_proof, vadcop_final_vk)
    }
}
