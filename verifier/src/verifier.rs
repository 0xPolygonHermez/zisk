use proofman_verifier::{verify_vadcop_final_compressed_u64, verify_vadcop_final_u64};

/// Length, in u64 words, of the Vadcop final verification key appended to a serialized proof.
pub const VADCOP_VK_LEN_WORDS: usize = 4;

/// Number of public values in a Zisk proof.
pub const ZISK_PUBLICS: usize = 64;

/// Length of the program VK in u64 elements (32 bytes / 8).
pub const PROGRAM_VK_LEN: usize = 4;

/// Expected `n_publics` header value: program VK + publics.
const EXPECTED_N_PUBLICS: u64 = (PROGRAM_VK_LEN + ZISK_PUBLICS) as u64;

pub fn verify_vadcop_final_proof(zisk_proof: &[u64], vadcop_final_vk: &[u64]) -> bool {
    // Format: [minimal(1)][n_publics(1)][publics(EXPECTED_N_PUBLICS)][proof]

    if zisk_proof.len() < (2 + EXPECTED_N_PUBLICS as usize) {
        return false;
    }

    if vadcop_final_vk.len() != PROGRAM_VK_LEN {
        return false;
    }

    let minimal = zisk_proof[0] == 1;
    let vadcop_proof = &zisk_proof[1..];

    if vadcop_proof[0] != EXPECTED_N_PUBLICS {
        return false;
    }

    if minimal {
        verify_vadcop_final_compressed_u64(vadcop_proof, vadcop_final_vk)
    } else {
        verify_vadcop_final_u64(vadcop_proof, vadcop_final_vk)
    }
}
