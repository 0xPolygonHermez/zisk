use proofman_verifier::{verify_vadcop_final_bytes, verify_vadcop_final_compressed_bytes};

pub fn verify_vadcop_final_proof(zisk_proof: &[u8], vadcop_final_vk: &[u8]) -> bool {
    // Format: [compressed(8)][pubs_len(8)][pubs][proof_bytes]

    // Read compressed flag (8 bytes, u64 little-endian)
    let compressed = u64::from_le_bytes([
        zisk_proof[0],
        zisk_proof[1],
        zisk_proof[2],
        zisk_proof[3],
        zisk_proof[4],
        zisk_proof[5],
        zisk_proof[6],
        zisk_proof[7],
    ]) == 1;

    let vadcop_proof = &zisk_proof[8..];

    if compressed {
        verify_vadcop_final_compressed_bytes(vadcop_proof, vadcop_final_vk)
    } else {
        verify_vadcop_final_bytes(vadcop_proof, vadcop_final_vk)
    }
}
