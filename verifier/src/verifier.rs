use proofman_verifier::VadcopFinalProof;

/// Length, in u64 words, of the Vadcop final verification key appended to a serialized proof.
pub const VADCOP_VK_LEN_WORDS: usize = 4;

/// Number of public values in a Zisk proof.
pub const ZISK_PUBLICS: usize = 64;

/// Length of the program VK in u64 elements (32 bytes / 8).
pub const PROGRAM_VK_LEN: usize = 4;

/// The program-level public count: program VK + user publics. This is the
/// compressed (minimal) vadcop_final proof's `n_publics`, and the width of the
/// `[vk | inputs]` view once the recursion-layer flag is stripped.
pub const PROGRAM_N_PUBLICS: usize = PROGRAM_VK_LEN + ZISK_PUBLICS; // 68

/// The (non-minimal) vadcop_final circuit emits a leading `is_vadcop_final_proof`
/// public at index 0 (see the recurser), so its STARK public vector is one slot
/// wider than the flag-free program view.
pub const VADCOP_FINAL_FLAG_LEN: usize = 1;

/// The value of the `is_vadcop_final_proof` public on a genuine vadcop_final
/// leaf: the circuit hard-codes `signal output is_vadcop_final_proof <== 1`. The
/// recurser reads this at public index 0 to classify leaf (1) vs aggregated (0).
pub const IS_VADCOP_FINAL_PROOF: u64 = 1;

/// Expected `n_publics` header value for a NON-minimal vadcop_final proof:
/// `is_vadcop_final_proof(1) | program VK(4) | publics(64)` = 69.
const EXPECTED_N_PUBLICS_FINAL: u64 = (VADCOP_FINAL_FLAG_LEN + PROGRAM_N_PUBLICS) as u64;

/// Expected `n_publics` header value for a minimal (compressed) proof: the
/// `final_compressed` circuit strips the flag, so it is flag-free = 68.
const EXPECTED_N_PUBLICS_COMPRESSED: u64 = PROGRAM_N_PUBLICS as u64;

pub fn verify_vadcop_final_proof(zisk_proof: &[u64], vadcop_final_vk: &[u64], hash: &str) -> bool {
    // Format: [minimal(1)][n_publics(1)][publics(n_publics)][proof]
    // n_publics is 69 for a full vadcop_final proof (flag @0) and 68 for a
    // minimal/compressed one (flag stripped by the FinalCompressed circuit).

    if zisk_proof.len() < 2 {
        return false;
    }

    if vadcop_final_vk.len() != PROGRAM_VK_LEN {
        return false;
    }

    let minimal = zisk_proof[0] == 1;
    let vadcop_proof = &zisk_proof[1..];

    let expected_n_publics =
        if minimal { EXPECTED_N_PUBLICS_COMPRESSED } else { EXPECTED_N_PUBLICS_FINAL };
    if zisk_proof.len() < 2 + expected_n_publics as usize {
        return false;
    }
    if vadcop_proof[0] != expected_n_publics {
        return false;
    }

    verify_by_family(hash, minimal, vadcop_proof, vadcop_final_vk)
}

/// Dispatch to the verifier for `hash`. See [`crate::blake3`] for why ZisK
/// commits its own rather than using proofman's.
fn verify_by_family(hash: &str, minimal: bool, vadcop_proof: &[u64], vk: &[u64]) -> bool {
    match hash {
        // blake3 builds no compressed stage, so no minimal proof is from a blake3 key.
        "blake3" if minimal => false,
        "blake3" => crate::blake3::vadcop_final::verify_u64(vadcop_proof, vk),
        "Poseidon1" if minimal => {
            crate::poseidon1::vadcop_final_compressed::verify_u64(vadcop_proof, vk)
        }
        "Poseidon1" => crate::poseidon1::vadcop_final::verify_u64(vadcop_proof, vk),
        "Poseidon2" if minimal => {
            crate::poseidon2::vadcop_final_compressed::verify_u64(vadcop_proof, vk)
        }
        "Poseidon2" => crate::poseidon2::vadcop_final::verify_u64(vadcop_proof, vk),
        _ => false,
    }
}

/// Host-side counterpart to [`verify_vadcop_final_proof`], dispatching on the
/// family and stage the proof declares.
pub fn verify_vadcop_final(proof: &VadcopFinalProof, vk: &[u64]) -> bool {
    match proof.hash.as_str() {
        "blake3" if proof.compressed => false,
        "blake3" => crate::blake3::vadcop_final::verify(proof, vk),
        "Poseidon1" if proof.compressed => {
            crate::poseidon1::vadcop_final_compressed::verify(proof, vk)
        }
        "Poseidon1" => crate::poseidon1::vadcop_final::verify(proof, vk),
        "Poseidon2" if proof.compressed => {
            crate::poseidon2::vadcop_final_compressed::verify(proof, vk)
        }
        "Poseidon2" => crate::poseidon2::vadcop_final::verify(proof, vk),
        _ => false,
    }
}

/// Serialized length in bytes a proof of this family and stage must have.
pub fn expected_proof_bytes(hash: &str, minimal: bool) -> Option<usize> {
    match hash {
        "blake3" if minimal => None,
        "blake3" => Some(crate::blake3::vadcop_final::expected_proof_bytes()),
        "Poseidon1" if minimal => {
            Some(crate::poseidon1::vadcop_final_compressed::expected_proof_bytes())
        }
        "Poseidon1" => Some(crate::poseidon1::vadcop_final::expected_proof_bytes()),
        "Poseidon2" if minimal => {
            Some(crate::poseidon2::vadcop_final_compressed::expected_proof_bytes())
        }
        "Poseidon2" => Some(crate::poseidon2::vadcop_final::expected_proof_bytes()),
        _ => None,
    }
}

/// Return the program-level publics `[program VK | inputs]` from a vadcop_final
/// publics vector, stripping the recursion-layer `is_vadcop_final_proof` flag
/// when present.
///
/// The flag is present iff the vector is `VADCOP_FINAL_FLAG_LEN` longer than the
/// flag-free `PROGRAM_N_PUBLICS` (i.e. a non-minimal vadcop_final proof, len 69).
/// Compressed proofs and already-stripped views (len 68) pass through unchanged.
/// Anything else is returned as-is (callers assert their own lengths).
pub fn program_publics(publics_full: &[u64]) -> &[u64] {
    if publics_full.len() == VADCOP_FINAL_FLAG_LEN + PROGRAM_N_PUBLICS {
        &publics_full[VADCOP_FINAL_FLAG_LEN..]
    } else {
        publics_full
    }
}
