use proofman_verifier::VadcopFinalProof;

/// Length, in u64 words, of the Vadcop final verification key appended to a serialized proof.
pub const VADCOP_VK_LEN_WORDS: usize = 4;

/// Length, in u64 words, of the hash-family tag appended after the verification key.
pub const HASH_TAG_LEN_WORDS: usize = 1;

/// Hash families, indexed by the tag written into a serialized proof. Position is the
/// wire value, so entries are append-only: renumbering reinterprets existing proofs.
const HASH_TAGS: [&str; 3] = ["Poseidon1", "Poseidon2", "blake3"];

/// Tag for a hash family id, or `None` if the family has no wire encoding.
pub fn hash_tag(hash_id: &str) -> Option<u64> {
    HASH_TAGS.iter().position(|&f| f == hash_id).map(|i| i as u64)
}

/// Hash family id for a tag read off a serialized proof, or `None` if unrecognized.
///
/// The tag is untrusted routing metadata: it only selects which verifier runs, and a wrong
/// choice fails against the caller's expected verification key, whose const-tree root is
/// computed under the real family's own hash.
pub fn hash_id_from_tag(tag: u64) -> Option<&'static str> {
    HASH_TAGS.get(tag as usize).copied()
}

/// Number of public values in a Zisk proof.
pub const ZISK_PUBLICS: usize = 64;

/// Length of the program VK in u64 elements (32 bytes / 8).
pub const PROGRAM_VK_LEN: usize = 4;

/// The program-level public count: program VK + user publics. This is the
/// compressed (minimal) vadcop_final proof's `n_publics`, and the width of the
/// `[vk | inputs]` view once the recursion-layer flag is stripped.
pub const PROGRAM_N_PUBLICS: usize = PROGRAM_VK_LEN + ZISK_PUBLICS; // 68

/// The (non-minimal) vadcop_final circuit emits a leading `is_vadcop_final_proof`
/// public at index 0, so its STARK public vector is one slot wider than the
/// flag-free program view.
pub const VADCOP_FINAL_FLAG_LEN: usize = 1;

/// The value of the `is_vadcop_final_proof` public on a genuine vadcop_final leaf.
/// The recurser reads this at public index 0 to classify leaf (1) vs aggregated (0).
pub const IS_VADCOP_FINAL_PROOF: u64 = 1;

/// The Goldilocks prime `p = 2^64 - 2^32 + 1`. A field element has exactly one
/// canonical encoding, `[0, p)`.
pub const GOLDILOCKS_ORDER: u64 = 0xFFFF_FFFF_0000_0001;

/// Whether every word is a canonical Goldilocks element (`< p`).
///
/// `x` and `x + p` are one field element to the STARK verifier but two different
/// application outputs (low 32 bits). Canonical encodings are what keep the verified
/// statement and the reported outputs the same thing.
pub fn publics_are_canonical(publics: &[u64]) -> bool {
    publics.iter().all(|&w| w < GOLDILOCKS_ORDER)
}

/// Expected `n_publics` header for a NON-minimal vadcop_final proof:
/// `is_vadcop_final_proof(1) | program VK(4) | publics(64)` = 69.
const EXPECTED_N_PUBLICS_FINAL: u64 = (VADCOP_FINAL_FLAG_LEN + PROGRAM_N_PUBLICS) as u64;

/// Expected `n_publics` header for a minimal (compressed) proof: the
/// `final_compressed` circuit strips the flag, so it is flag-free = 68.
const EXPECTED_N_PUBLICS_COMPRESSED: u64 = PROGRAM_N_PUBLICS as u64;

/// The exact public count a proof of this stage must carry.
///
/// The generated `q_verify` indexes fixed public slots — through 68 for the flagged
/// stage, 67 for the compressed one — so anything shorter panics rather than failing.
/// `stark_verify`'s own length check is self-consistent with the count the proof
/// declares, so it does not catch this; only pinning the count does.
pub const fn expected_n_publics(minimal: bool) -> usize {
    if minimal {
        EXPECTED_N_PUBLICS_COMPRESSED as usize
    } else {
        EXPECTED_N_PUBLICS_FINAL as usize
    }
}

/// The `minimal` marker a serialized proof carries, or `None` if it is not a boolean.
///
/// The marker sits outside the STARK payload, so nothing downstream would catch a
/// garbage value: without this, a valid 69-public proof passes carrying marker `2`.
fn minimal_marker(zisk_proof: &[u64]) -> Option<bool> {
    match *zisk_proof.first()? {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

/// Whether a serialized proof is an aggregated fold rather than a leaf, from the
/// `is_vadcop_final_proof` public. A compressed proof no longer carries it: `None`.
pub fn committed_is_aggregate(zisk_proof: &[u64]) -> Option<bool> {
    if zisk_proof.len() < 2 || minimal_marker(zisk_proof)? {
        return None;
    }
    if zisk_proof[1] != EXPECTED_N_PUBLICS_FINAL {
        return None;
    }
    if zisk_proof.len() < 2 + EXPECTED_N_PUBLICS_FINAL as usize {
        return None;
    }
    Some(zisk_proof[2] != IS_VADCOP_FINAL_PROOF)
}

/// The program VK limbs a serialized proof commits to, or `None` if malformed.
///
/// This is the identity the proof claims (ROM root for a leaf, recursion domain for an
/// aggregate), orthogonal to the `vadcop_final_vk` the STARK is checked against.
pub fn committed_program_vk(zisk_proof: &[u64]) -> Option<&[u64]> {
    if zisk_proof.len() < 2 {
        return None;
    }
    let minimal = minimal_marker(zisk_proof)?;
    let expected_n_publics =
        if minimal { EXPECTED_N_PUBLICS_COMPRESSED } else { EXPECTED_N_PUBLICS_FINAL };
    if zisk_proof[1] != expected_n_publics {
        return None;
    }
    let n = expected_n_publics as usize;
    if zisk_proof.len() < 2 + n {
        return None;
    }
    Some(&program_publics(&zisk_proof[2..2 + n])[..PROGRAM_VK_LEN])
}

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

    // Strictly boolean: the marker is outside the STARK payload, so a garbage value
    // would otherwise select the full-proof path and verify.
    let Some(minimal) = minimal_marker(zisk_proof) else {
        return false;
    };
    let vadcop_proof = &zisk_proof[1..];

    let expected_n_publics = expected_n_publics(minimal);
    if zisk_proof.len() < 2 + expected_n_publics {
        return false;
    }
    if vadcop_proof[0] != expected_n_publics as u64 {
        return false;
    }

    if !publics_are_canonical(&vadcop_proof[1..1 + expected_n_publics]) {
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
    // `stark_verify` reads exactly `vk[0..4]`, so a longer key would be silently
    // truncated to one the caller never pinned. Exact, not `>=`.
    if vk.len() != PROGRAM_VK_LEN {
        return false;
    }
    if proof.public_values.len() != expected_n_publics(proof.compressed) {
        return false;
    }
    if !publics_are_canonical(&proof.public_values) {
        return false;
    }
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
/// Anything else is returned as-is (callers assert their own lengths).
pub fn program_publics(publics_full: &[u64]) -> &[u64] {
    if publics_full.len() == VADCOP_FINAL_FLAG_LEN + PROGRAM_N_PUBLICS {
        &publics_full[VADCOP_FINAL_FLAG_LEN..]
    } else {
        publics_full
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;

    const HDR: usize = 2;
    const LEAF_LEN: usize = HDR + EXPECTED_N_PUBLICS_FINAL as usize;

    /// `[minimal][n_publics][flag | vk(4) | inputs]`, the prefix the classifiers read.
    fn leaf(flag: u64, vk: [u64; PROGRAM_VK_LEN]) -> [u64; LEAF_LEN] {
        let mut v = [7u64; LEAF_LEN];
        v[0] = 0;
        v[1] = EXPECTED_N_PUBLICS_FINAL;
        v[2] = flag;
        v[3..3 + PROGRAM_VK_LEN].copy_from_slice(&vk);
        v
    }

    #[test]
    fn a_leaf_is_not_an_aggregate() {
        let p = leaf(IS_VADCOP_FINAL_PROOF, [1, 2, 3, 4]);
        assert_eq!(committed_is_aggregate(&p), Some(false));
        assert_eq!(committed_program_vk(&p), Some(&[1u64, 2, 3, 4][..]));
    }

    #[test]
    fn a_flag_zero_proof_is_an_aggregate() {
        let p = leaf(0, [9, 9, 9, 9]);
        assert_eq!(committed_is_aggregate(&p), Some(true));
        assert_eq!(committed_program_vk(&p), Some(&[9u64, 9, 9, 9][..]));
    }

    /// Compression strips the flag, which is why an aggregate is refused compression.
    #[test]
    fn a_minimal_proof_cannot_be_classified() {
        let mut p = leaf(0, [1, 2, 3, 4]);
        p[0] = 1;
        p[1] = EXPECTED_N_PUBLICS_COMPRESSED;
        assert_eq!(committed_is_aggregate(&p), None);
    }

    #[test]
    fn a_truncated_proof_classifies_as_nothing() {
        assert_eq!(committed_is_aggregate(&[]), None);
        assert_eq!(committed_is_aggregate(&[0]), None);
        assert_eq!(committed_is_aggregate(&[0, EXPECTED_N_PUBLICS_FINAL]), None);
        assert_eq!(committed_is_aggregate(&[0, 12, 0, 1, 2, 3, 4]), None);
    }

    /// Every family the enum knows must round-trip through its wire tag.
    #[test]
    fn every_tag_round_trips() {
        for (i, f) in HASH_TAGS.iter().enumerate() {
            assert_eq!(hash_tag(f), Some(i as u64));
            assert_eq!(hash_id_from_tag(i as u64), Some(*f));
        }
        assert_eq!(hash_id_from_tag(HASH_TAGS.len() as u64), None);
        assert_eq!(hash_tag("poseidon3"), None);
    }

    fn final_proof(publics: Vec<u64>, compressed: bool) -> VadcopFinalProof {
        VadcopFinalProof::new(vec![0u64; 8], publics, compressed, "Poseidon2".to_string())
    }

    /// `q_verify` indexes fixed public slots, so a short vector would panic rather than
    /// fail. `stark_verify` cannot catch it: its length check follows the declared count.
    #[test]
    fn a_short_publics_vector_is_refused_before_dispatch() {
        let short = final_proof(vec![0; 10], false);
        assert!(!verify_vadcop_final(&short, &[1, 2, 3, 4]));

        // The compressed stage is one narrower, so the flagged width is wrong for it too.
        let flagged = final_proof(vec![0; EXPECTED_N_PUBLICS_FINAL as usize], true);
        assert!(!verify_vadcop_final(&flagged, &[1, 2, 3, 4]));
    }

    /// `stark_verify` reads exactly `vk[0..4]`; a longer key must be refused, not truncated.
    #[test]
    fn a_wrong_length_setup_key_is_refused() {
        let publics = vec![0; EXPECTED_N_PUBLICS_FINAL as usize];
        assert!(!verify_vadcop_final(&final_proof(publics.clone(), false), &[1, 2, 3]));
        assert!(!verify_vadcop_final(&final_proof(publics, false), &[1, 2, 3, 4, 5]));
    }

    #[test]
    fn a_non_canonical_public_is_refused_on_the_host_path() {
        let mut publics = vec![0u64; EXPECTED_N_PUBLICS_FINAL as usize];
        publics[VADCOP_FINAL_FLAG_LEN + PROGRAM_VK_LEN] = GOLDILOCKS_ORDER;
        assert!(!verify_vadcop_final(&final_proof(publics, false), &[1, 2, 3, 4]));
    }

    /// The marker sits outside the STARK payload, so a garbage value must be refused
    /// here or an otherwise valid full proof verifies while declaring nonsense.
    #[test]
    fn a_non_boolean_minimal_marker_is_refused() {
        let mut p = leaf(IS_VADCOP_FINAL_PROOF, [1, 2, 3, 4]).to_vec();
        p[0] = 2;
        assert!(!verify_vadcop_final_proof(&p, &[1, 2, 3, 4], "Poseidon2"));
        assert_eq!(committed_program_vk(&p), None);
        assert_eq!(committed_is_aggregate(&p), None);
    }

    /// A non-canonical public must be refused before it reaches the STARK verifier.
    #[test]
    fn a_non_canonical_public_is_refused() {
        let mut p = leaf(IS_VADCOP_FINAL_PROOF, [1, 2, 3, 4]).to_vec();
        p[2 + VADCOP_FINAL_FLAG_LEN + PROGRAM_VK_LEN] = GOLDILOCKS_ORDER;
        assert!(!verify_vadcop_final_proof(&p, &[1, 2, 3, 4], "Poseidon2"));
    }
}
