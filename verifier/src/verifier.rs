use alloc::string::String;

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
    first_non_canonical(publics).is_none()
}

/// Index of the first word that is not a canonical Goldilocks element.
pub(crate) fn first_non_canonical(words: &[u64]) -> Option<usize> {
    words.iter().position(|&w| w >= GOLDILOCKS_ORDER)
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

/// The flagged stage's slot-0 value as a classification: 1 = leaf, 0 = fold. The circuit
/// emits only those two, so anything else is malformed rather than a third kind.
pub(crate) fn is_aggregate_flag(flag: u64) -> Option<bool> {
    match flag {
        IS_VADCOP_FINAL_PROOF => Some(false),
        0 => Some(true),
        _ => None,
    }
}

/// The generated entry points for one (family, stage) pair.
///
/// Function pointers rather than a trait: each stage is its own generated module, so
/// there is no shared type to implement one on.
struct StageVerifier {
    verify: fn(&VadcopFinalProof, &[u64]) -> bool,
    /// The same check over `[n_publics][publics][proof]` in place, without a copy.
    verify_u64: fn(&[u64], &[u64]) -> bool,
    expected_proof_bytes: fn() -> usize,
    /// The setup's key for this stage, from [`crate::keys`].
    vk: [u64; PROGRAM_VK_LEN],
}

/// The single routing table. Every dispatch below goes through it, so adding a family
/// or a stage is one edit rather than three that can drift apart.
///
/// `None` means that pair has no verifier: an unknown family, or blake3 compressed —
/// blake3 proving keys are built without the `vadcop_final_compressed` stage, so no
/// minimal proof can come from one. See [`crate::blake3`] for why ZisK commits its own
/// verifiers rather than using proofman's.
fn stage_verifier(hash: &str, minimal: bool) -> Option<StageVerifier> {
    macro_rules! stage {
        ($module:path, $vk:expr) => {{
            use $module as m;
            StageVerifier {
                verify: m::verify,
                verify_u64: m::verify_u64,
                expected_proof_bytes: m::expected_proof_bytes,
                vk: $vk,
            }
        }};
    }
    use crate::keys::*;

    Some(match (hash, minimal) {
        ("blake3", false) => stage!(crate::blake3::vadcop_final, VADCOP_FINAL_VK_BLAKE3),
        ("Poseidon1", false) => {
            stage!(crate::poseidon1::vadcop_final, VADCOP_FINAL_VK_POSEIDON1)
        }
        ("Poseidon1", true) => {
            stage!(crate::poseidon1::vadcop_final_compressed, VADCOP_FINAL_COMPRESSED_VK_POSEIDON1)
        }
        ("Poseidon2", false) => {
            stage!(crate::poseidon2::vadcop_final, VADCOP_FINAL_VK_POSEIDON2)
        }
        ("Poseidon2", true) => {
            stage!(crate::poseidon2::vadcop_final_compressed, VADCOP_FINAL_COMPRESSED_VK_POSEIDON2)
        }
        _ => return None,
    })
}

/// [`verify`] as a bool, for callers that only need yes or no.
pub fn verify_vadcop_final(proof: &VadcopFinalProof, vk: &[u64]) -> bool {
    // `stark_verify` reads exactly `vk[0..4]`, so a longer key would be silently
    // truncated to one the caller never pinned. Exact, not `>=`.
    <&[u64; PROGRAM_VK_LEN]>::try_from(vk).is_ok_and(|vk| verify(proof, vk).is_ok())
}

/// What a verified proof proves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    /// The program identity: ROM root for a leaf, recursion domain for an aggregate.
    pub program_vk: [u64; PROGRAM_VK_LEN],
    /// The 64 public outputs, as canonical field elements.
    pub outputs: [u64; ZISK_PUBLICS],
    /// `Some(true)` for a recurser fold, `Some(false)` for a leaf, `None` for a compressed
    /// proof, whose stage strips the flag. Producers refuse to compress an aggregate.
    pub is_aggregate: Option<bool>,
}

impl Verified {
    /// The outputs as the guest committed them. Always `Some` unless `is_aggregate` is
    /// `Some(true)`: [`verify`] checks every leaf, compressed ones included.
    pub fn outputs_u32(&self) -> Option<[u32; ZISK_PUBLICS]> {
        let mut out = [0u32; ZISK_PUBLICS];
        for (o, &w) in out.iter_mut().zip(&self.outputs) {
            *o = u32::try_from(w).ok()?;
        }
        Some(out)
    }
}

/// Why [`verify`] rejected a proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// No verifier for this family and stage (unknown family, or blake3 compressed).
    UnsupportedStage { hash: String, compressed: bool },
    /// The publics vector is not the width this stage carries.
    PublicCount { expected: usize, got: usize },
    /// A public is not a canonical Goldilocks element.
    NonCanonicalPublic { index: usize },
    /// The leaf/fold flag is neither 1 (leaf) nor 0 (fold).
    InvalidLeafFlag(u64),
    /// The STARK payload is not the length this stage's proofs have.
    ProofLength { expected_bytes: usize, got_bytes: usize },
    /// A leaf output does not fit in u32, which a ZisK guest cannot commit.
    OutputNotU32 { index: usize },
    /// An aggregate whose committed domain is not the key it verifies under: a subtree
    /// folded by a different recurser.
    AggregateDomainMismatch { declared: [u64; PROGRAM_VK_LEN] },
    /// The proof is for a different program than the caller expects.
    ProgramMismatch { committed: [u64; PROGRAM_VK_LEN] },
    /// The STARK itself does not verify against the given key.
    InvalidProof,
    /// The bytes are not a well-formed flat proof ([`verify_saved_words`] only).
    Malformed(crate::DecodeError),
}

impl core::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedStage { hash, compressed } => {
                write!(f, "no verifier for hash family {hash:?} (compressed: {compressed})")
            }
            Self::PublicCount { expected, got } => {
                write!(f, "{got} publics, the stage carries {expected}")
            }
            Self::NonCanonicalPublic { index } => {
                write!(f, "public {index} is not a canonical Goldilocks element")
            }
            Self::InvalidLeafFlag(v) => write!(f, "leaf flag {v} is not 0 or 1"),
            Self::ProofLength { expected_bytes, got_bytes } => {
                write!(f, "proof is {got_bytes} bytes, the stage's proofs are {expected_bytes}")
            }
            Self::OutputNotU32 { index } => write!(f, "leaf output {index} does not fit in u32"),
            Self::AggregateDomainMismatch { declared } => write!(
                f,
                "aggregate declares recursion domain {declared:?} but verifies under another \
                 key; its subtree was not produced by this recurser"
            ),
            Self::ProgramMismatch { committed } => {
                write!(f, "proof is for program {committed:?}, not the expected one")
            }
            Self::InvalidProof => write!(f, "STARK verification failed"),
            Self::Malformed(e) => write!(f, "malformed proof: {e}"),
        }
    }
}

impl core::error::Error for VerifyError {}

/// Verify a vadcop_final proof against `vadcop_vk` and return what it proves.
///
/// `vadcop_vk` must be a key the caller trusts (a release constant from [`vadcop_vk`], or
/// the recurser's own key for an aggregate), never one read off the proof. Callers still
/// check [`Verified::program_vk`] against the program they expect.
pub fn verify(
    proof: &VadcopFinalProof,
    vadcop_vk: &[u64; PROGRAM_VK_LEN],
) -> Result<Verified, VerifyError> {
    let shape = Shape {
        hash: &proof.hash,
        compressed: proof.compressed,
        publics: &proof.public_values,
        proof_words: proof.proof.len(),
    };
    let (stage, verified) = check_shape(&shape, vadcop_vk, None)?;
    if !(stage.verify)(proof, vadcop_vk) {
        return Err(VerifyError::InvalidProof);
    }
    Ok(verified)
}

/// [`verify`] over the flat layout in place, as a guest receives it: no copy of the proof.
///
/// With `expected_program_vk`, a proof for any other program is refused before the STARK
/// runs, so a guest pays a few comparisons, not a full verification, for it.
pub fn verify_saved_words(
    words: &[u64],
    vadcop_vk: &[u64; PROGRAM_VK_LEN],
    expected_program_vk: Option<&[u64; PROGRAM_VK_LEN]>,
) -> Result<Verified, VerifyError> {
    let view = crate::decode::split_saved(words).map_err(VerifyError::Malformed)?;
    let shape = Shape {
        hash: view.hash,
        compressed: view.compressed,
        publics: view.publics,
        proof_words: view.proof.len(),
    };
    let (stage, verified) = check_shape(&shape, vadcop_vk, expected_program_vk)?;
    if !(stage.verify_u64)(view.stark_body, vadcop_vk) {
        return Err(VerifyError::InvalidProof);
    }
    Ok(verified)
}

/// What every check reads, borrowed from wherever the proof lives.
struct Shape<'a> {
    hash: &'a str,
    compressed: bool,
    publics: &'a [u64],
    proof_words: usize,
}

/// Every check before the STARK, cheapest first, and the statement the proof makes if
/// the STARK then holds.
fn check_shape(
    s: &Shape,
    vadcop_vk: &[u64; PROGRAM_VK_LEN],
    expected_program_vk: Option<&[u64; PROGRAM_VK_LEN]>,
) -> Result<(StageVerifier, Verified), VerifyError> {
    let stage = stage_verifier(s.hash, s.compressed).ok_or_else(|| {
        VerifyError::UnsupportedStage { hash: s.hash.into(), compressed: s.compressed }
    })?;
    let expected = expected_n_publics(s.compressed);
    if s.publics.len() != expected {
        return Err(VerifyError::PublicCount { expected, got: s.publics.len() });
    }
    if let Some(index) = first_non_canonical(s.publics) {
        return Err(VerifyError::NonCanonicalPublic { index });
    }
    // `VadcopFinalProof` is public, so a caller can hand us a flagged proof that never
    // went through `new_from_vadcop_proof`'s strict flag check.
    let is_aggregate = if s.compressed {
        None
    } else {
        let flag = s.publics[0];
        Some(is_aggregate_flag(flag).ok_or(VerifyError::InvalidLeafFlag(flag))?)
    };
    let publics = program_publics(s.publics);
    let verified = Verified {
        program_vk: publics[..PROGRAM_VK_LEN].try_into().unwrap(),
        outputs: publics[PROGRAM_VK_LEN..].try_into().unwrap(),
        is_aggregate,
    };
    if expected_program_vk.is_some_and(|vk| *vk != verified.program_vk) {
        return Err(VerifyError::ProgramMismatch { committed: verified.program_vk });
    }
    if is_aggregate == Some(true) {
        // An aggregate's declared domain must be the key it verifies under, or a subtree
        // from another recurser rides through.
        if verified.program_vk != *vadcop_vk {
            return Err(VerifyError::AggregateDomainMismatch { declared: verified.program_vk });
        }
    } else if let Some(index) = verified.outputs.iter().position(|&w| w > u32::MAX as u64) {
        // A leaf, compressed ones included: aggregates are never compressed.
        return Err(VerifyError::OutputNotU32 { index });
    }
    let expected_bytes = (stage.expected_proof_bytes)();
    if s.proof_words * 8 != expected_bytes {
        return Err(VerifyError::ProofLength { expected_bytes, got_bytes: s.proof_words * 8 });
    }
    Ok((stage, verified))
}

/// Serialized length in bytes a proof of this family and stage must have, or `None` if
/// no such stage exists — see [`stage_verifier`].
pub fn expected_proof_bytes(hash: &str, minimal: bool) -> Option<usize> {
    stage_verifier(hash, minimal).map(|s| (s.expected_proof_bytes)())
}

/// The setup's vadcop_final key for this family and stage, or `None` if this crate has
/// none for it. Selecting by a proof's own `hash`/`compressed` is safe: the key comes
/// from the crate, the proof only picks which. Leaf proofs only; a recurser proof
/// verifies under its recurser's key.
pub fn vadcop_vk(hash: &str, compressed: bool) -> Option<[u64; PROGRAM_VK_LEN]> {
    stage_verifier(hash, compressed).map(|s| s.vk)
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

    /// The `/8` word conversion in the length gate assumes a byte count that is a whole
    /// number of u64 words; pin that for every stage the keys actually build.
    #[test]
    fn every_built_stage_has_a_whole_word_proof_size() {
        for (hash, minimal) in [
            ("blake3", false),
            ("Poseidon1", false),
            ("Poseidon1", true),
            ("Poseidon2", false),
            ("Poseidon2", true),
        ] {
            let bytes = expected_proof_bytes(hash, minimal)
                .unwrap_or_else(|| panic!("{hash} minimal={minimal} should have a stage"));
            assert_eq!(bytes % 8, 0, "{hash} minimal={minimal}");
            assert!(bytes > 0, "{hash} minimal={minimal}");
        }
        // blake3 builds no compressed stage.
        assert_eq!(expected_proof_bytes("blake3", true), None);
        assert_eq!(expected_proof_bytes("poseidon3", false), None);
    }

    /// The circuit emits slot 0 as 1 (leaf) or 0 (fold) and nothing else, so a third
    /// value is malformed, not a third classification.
    #[test]
    fn the_leaf_flag_is_strictly_boolean() {
        assert_eq!(is_aggregate_flag(IS_VADCOP_FINAL_PROOF), Some(false));
        assert_eq!(is_aggregate_flag(0), Some(true));
        assert_eq!(is_aggregate_flag(2), None);

        let mut publics = vec![0u64; EXPECTED_N_PUBLICS_FINAL as usize];
        publics[0] = 2;
        assert!(!verify_vadcop_final(&final_proof(publics, false), &[1, 2, 3, 4]));
    }

    /// Every published key is a canonical field element per limb, and no two stages share
    /// one (a copy-paste between stages would otherwise go unnoticed).
    #[test]
    fn every_published_key_is_canonical_and_distinct() {
        let stages = [
            ("blake3", false),
            ("Poseidon1", false),
            ("Poseidon1", true),
            ("Poseidon2", false),
            ("Poseidon2", true),
        ];
        let keys: Vec<_> = stages.iter().filter_map(|&(h, c)| vadcop_vk(h, c)).collect();
        assert!(!keys.is_empty());
        for (i, k) in keys.iter().enumerate() {
            assert!(publics_are_canonical(k), "{k:?}");
            assert!(!keys[..i].contains(k), "{k:?} appears twice");
        }
        assert_eq!(vadcop_vk("blake3", true), None);
    }
}
