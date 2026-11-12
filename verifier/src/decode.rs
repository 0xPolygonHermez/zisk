//! Decoding proofs and keys from bytes.
//!
//! ZisK proofs have one byte format, the flat layout read by [`decode_saved`]: what
//! `cargo-zisk prove` saves, what `Proof::get_proof_bytes` returns and what guests verify.
//! A saved file may end with a [seed trailer](SEED_TRAILER_TAG): unauthenticated metadata, not
//! part of the proof. The decoders and [`crate::verify_saved`] strip it;
//! [`crate::verify_saved_words`], what guests call, refuses it, so a guest takes one encoding
//! per proof.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use proofman_verifier::VadcopFinalProof;

use crate::verifier::{first_non_canonical, is_aggregate_flag};
use crate::{expected_n_publics, hash_id_from_tag, HASH_TAG_LEN_WORDS, VADCOP_VK_LEN_WORDS};

/// Ends a flat proof followed by the host randomness seed of the execution it attests to:
/// `[flat proof][seed(4)][SEED_TRAILER_TAG]`. No hash tag takes this value, so a file
/// without the trailer is never read as one with it. The seed is unauthenticated metadata.
pub const SEED_TRAILER_TAG: u64 = u64::from_le_bytes(*b"ZISKSEED");

/// Words in a seed: 256 bits.
pub const SEED_LEN_WORDS: usize = 4;

/// The trailer that appends `seed` to a flat proof.
pub fn seed_trailer(seed: [u64; SEED_LEN_WORDS]) -> [u64; SEED_LEN_WORDS + 1] {
    let [a, b, c, d] = seed;
    [a, b, c, d, SEED_TRAILER_TAG]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The input ends before a field it declares.
    Truncated,
    /// A byte length that is not a whole number of u64 words.
    NotWordAligned(usize),
    /// The `minimal` marker is neither 0 nor 1.
    InvalidMinimal(u64),
    /// The hash-family tag has no family.
    UnknownHashTag(u64),
    /// `n_publics` is not the count the declared stage carries.
    PublicCount { expected: usize, got: u64 },
    /// A public is not a canonical Goldilocks element.
    NonCanonicalPublic { index: usize },
    /// The leaf/fold flag of an uncompressed proof is neither 1 (leaf) nor 0 (fold).
    InvalidLeafFlag(u64),
    /// A key that is not 4 u64 words.
    KeyLength(usize),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => write!(f, "input ends before the proof does"),
            Self::NotWordAligned(n) => write!(f, "{n} bytes is not a whole number of u64 words"),
            Self::InvalidMinimal(v) => write!(f, "minimal marker {v} is not 0 or 1"),
            Self::UnknownHashTag(t) => write!(f, "unknown hash family tag {t}"),
            Self::PublicCount { expected, got } => {
                write!(f, "n_publics is {got}, the stage carries {expected}")
            }
            Self::NonCanonicalPublic { index } => {
                write!(f, "public {index} is not a canonical Goldilocks element")
            }
            Self::InvalidLeafFlag(v) => write!(f, "leaf flag {v} is not 0 or 1"),
            Self::KeyLength(n) => write!(
                f,
                "a key is {} bytes ({VADCOP_VK_LEN_WORDS} little-endian u64 words), got {n}",
                VADCOP_VK_LEN_WORDS * 8
            ),
        }
    }
}

impl core::error::Error for DecodeError {}

/// Decode a key from its `verkey.bin`: 4 little-endian u64 words.
pub fn decode_vk(bytes: &[u8]) -> Result<[u64; VADCOP_VK_LEN_WORDS], DecodeError> {
    let bytes: &[u8; VADCOP_VK_LEN_WORDS * 8] =
        bytes.try_into().map_err(|_| DecodeError::KeyLength(bytes.len()))?;
    Ok(core::array::from_fn(|i| u64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().unwrap())))
}

/// A proof read from the flat layout.
#[derive(Debug, Clone)]
pub struct SavedProof {
    pub proof: VadcopFinalProof,
    /// The vadcop_final key the producer says it proved under. Untrusted: use it to pick
    /// among keys you already trust or to explain a mismatch, never as the key to verify
    /// against. A proof checked against the key it ships with proves nothing.
    pub claimed_vadcop_vk: [u64; VADCOP_VK_LEN_WORDS],
    /// The seed trailer's words, if the file has one.
    pub random_seed: Option<[u64; SEED_LEN_WORDS]>,
}

/// Decode the flat layout:
/// `[minimal][n_publics][publics][proof][vadcop_final_vk(4)][hash tag]`, u64 LE words,
/// optionally followed by the [seed trailer](SEED_TRAILER_TAG).
pub fn decode_saved(bytes: &[u8]) -> Result<SavedProof, DecodeError> {
    let mut words = words_of(bytes)?;
    let (flat, random_seed) = strip_seed_trailer(&words)?;
    let v = split_saved(flat)?;
    let (compressed, hash, claimed_vadcop_vk) = (v.compressed, v.hash, v.claimed_vadcop_vk);
    let public_values = v.publics.to_vec();
    // The proof is one contiguous run of `words`: keep it in place rather than copy it.
    let start = 2 + public_values.len();
    let end = start + v.proof.len();
    words.truncate(end);
    words.drain(..start);
    Ok(SavedProof {
        proof: VadcopFinalProof::new(words, public_values, compressed, String::from(hash)),
        claimed_vadcop_vk,
        random_seed,
    })
}

/// [`decode_saved`] over words already in memory. To verify them, prefer
/// [`crate::verify_saved_words`], which does not copy the proof.
pub fn decode_saved_words(words: &[u64]) -> Result<SavedProof, DecodeError> {
    let (flat, random_seed) = strip_seed_trailer(words)?;
    let v = split_saved(flat)?;
    Ok(SavedProof {
        proof: VadcopFinalProof::new(
            v.proof.to_vec(),
            v.publics.to_vec(),
            v.compressed,
            String::from(v.hash),
        ),
        claimed_vadcop_vk: v.claimed_vadcop_vk,
        random_seed,
    })
}

/// The flat layout's parts, borrowed in place.
pub(crate) struct SavedView<'a> {
    pub compressed: bool,
    pub hash: &'static str,
    pub publics: &'a [u64],
    pub proof: &'a [u64],
    /// `[n_publics][publics][proof]`, what the generated `verify_u64` reads.
    pub stark_body: &'a [u64],
    pub claimed_vadcop_vk: [u64; VADCOP_VK_LEN_WORDS],
}

/// `bytes` as little-endian u64 words.
pub(crate) fn words_of(bytes: &[u8]) -> Result<Vec<u64>, DecodeError> {
    if bytes.len() % 8 != 0 {
        return Err(DecodeError::NotWordAligned(bytes.len()));
    }
    Ok(bytes.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect())
}

/// Split off the [seed trailer](SEED_TRAILER_TAG), if `words` end with one.
pub(crate) fn strip_seed_trailer(
    words: &[u64],
) -> Result<(&[u64], Option<[u64; SEED_LEN_WORDS]>), DecodeError> {
    match words {
        [flat @ .., a, b, c, d, SEED_TRAILER_TAG] => Ok((flat, Some([*a, *b, *c, *d]))),
        [.., SEED_TRAILER_TAG] => Err(DecodeError::Truncated),
        _ => Ok((words, None)),
    }
}

/// The bare flat layout; a seed trailer reads as an unknown hash tag.
pub(crate) fn split_saved(words: &[u64]) -> Result<SavedView<'_>, DecodeError> {
    const TAIL: usize = VADCOP_VK_LEN_WORDS + HASH_TAG_LEN_WORDS;
    if words.len() < 2 + TAIL {
        return Err(DecodeError::Truncated);
    }
    let (body, tail) = words.split_at(words.len() - TAIL);
    let tag = tail[VADCOP_VK_LEN_WORDS];
    let hash = hash_id_from_tag(tag).ok_or(DecodeError::UnknownHashTag(tag))?;

    let compressed = match body[0] {
        0 => false,
        1 => true,
        v => return Err(DecodeError::InvalidMinimal(v)),
    };
    let expected = expected_n_publics(compressed);
    if body[1] != expected as u64 {
        return Err(DecodeError::PublicCount { expected, got: body[1] });
    }
    if body.len() < 2 + expected {
        return Err(DecodeError::Truncated);
    }
    let (publics, proof) = body[2..].split_at(expected);
    if let Some(index) = first_non_canonical(publics) {
        return Err(DecodeError::NonCanonicalPublic { index });
    }
    if !compressed && is_aggregate_flag(publics[0]).is_none() {
        return Err(DecodeError::InvalidLeafFlag(publics[0]));
    }
    Ok(SavedView {
        compressed,
        hash,
        publics,
        proof,
        stark_body: &body[1..],
        claimed_vadcop_vk: tail[..VADCOP_VK_LEN_WORDS].try_into().unwrap(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hash_tag, proof_size, verify, VerifyError, GOLDILOCKS_ORDER, IS_VADCOP_FINAL_PROOF,
    };
    use alloc::vec;

    #[test]
    fn a_key_is_4_little_endian_words() {
        let bytes: Vec<u8> = [1u64, 2, 3, u64::MAX].iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(decode_vk(&bytes), Ok([1, 2, 3, u64::MAX]));
        assert_eq!(decode_vk(&bytes[..31]), Err(DecodeError::KeyLength(31)));
        assert_eq!(decode_vk(&[0; 40]), Err(DecodeError::KeyLength(40)));
    }

    fn saved(minimal: bool, n_publics: usize, proof_words: usize, tag: u64) -> Vec<u8> {
        let mut words = vec![minimal as u64, n_publics as u64];
        // Slot 0 is the leaf flag on an uncompressed proof.
        words.extend((0..n_publics as u64).map(|i| if i == 0 && !minimal { 1 } else { i + 10 }));
        words.extend((0..proof_words as u64).map(|i| i + 1000));
        words.extend([7, 8, 9, 10, tag]);
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    #[test]
    fn decode_saved_splits_the_flat_layout() {
        let tag = hash_tag("Poseidon2").unwrap();
        let n = expected_n_publics(false);
        let bytes = saved(false, n, 5, tag);
        let s = decode_saved(&bytes).unwrap();
        assert!(!s.proof.compressed);
        assert_eq!(s.proof.hash, "Poseidon2");
        assert_eq!(s.proof.public_values.len(), n);
        assert_eq!(s.proof.public_values[..2], [1, 11]);
        assert_eq!(s.proof.proof, vec![1000, 1001, 1002, 1003, 1004]);
        assert_eq!(s.claimed_vadcop_vk, [7, 8, 9, 10]);

        let words: Vec<u64> =
            bytes.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
        assert_eq!(decode_saved_words(&words).unwrap().proof.proof, s.proof.proof);

        let n = expected_n_publics(true);
        let s = decode_saved(&saved(true, n, 1, hash_tag("blake3").unwrap())).unwrap();
        assert!(s.proof.compressed);
        assert_eq!(s.proof.hash, "blake3");
    }

    #[test]
    fn decode_saved_rejects_malformed_layouts() {
        let tag = hash_tag("Poseidon2").unwrap();
        let n = expected_n_publics(false);
        let good = saved(false, n, 5, tag);
        let err = |b: &[u8]| decode_saved(b).unwrap_err();

        assert_eq!(err(&good[..good.len() - 1]), DecodeError::NotWordAligned(good.len() - 1));
        assert_eq!(err(&saved(false, n, 5, 99)), DecodeError::UnknownHashTag(99));
        assert_eq!(
            err(&saved(false, n - 1, 5, tag)),
            DecodeError::PublicCount { expected: n, got: n as u64 - 1 }
        );
        assert_eq!(
            err(&saved(true, n, 5, tag)),
            DecodeError::PublicCount { expected: n - 1, got: n as u64 }
        );
        assert_eq!(err(&[]), DecodeError::Truncated);

        let mut bad = good.clone();
        bad[0] = 2;
        assert_eq!(err(&bad), DecodeError::InvalidMinimal(2));

        let mut bad = good.clone();
        bad[16 + 3 * 8..16 + 4 * 8].copy_from_slice(&GOLDILOCKS_ORDER.to_le_bytes());
        assert_eq!(err(&bad), DecodeError::NonCanonicalPublic { index: 3 });

        let mut bad = good.clone();
        bad[16..24].copy_from_slice(&2u64.to_le_bytes());
        assert_eq!(err(&bad), DecodeError::InvalidLeafFlag(2));

        // Declares 69 publics but carries fewer words than that.
        let short: Vec<u8> =
            [0u64, n as u64, 1, 2, 7, 7, 7, 7, tag].iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(err(&short), DecodeError::Truncated);
    }

    #[test]
    fn decode_saved_reads_the_seed_trailer() {
        let tag = hash_tag("Poseidon2").unwrap();
        let plain = saved(false, expected_n_publics(false), 5, tag);
        let mut with_seed = plain.clone();
        for w in seed_trailer([1, 2, 3, 4]) {
            with_seed.extend_from_slice(&w.to_le_bytes());
        }

        let s = decode_saved(&with_seed).unwrap();
        assert_eq!(s.random_seed, Some([1, 2, 3, 4]));
        assert_eq!(s.proof.proof, decode_saved(&plain).unwrap().proof.proof);
        assert_eq!(decode_saved(&plain).unwrap().random_seed, None);

        // The in-place path a guest takes reads only the bare layout.
        let words: Vec<u64> =
            with_seed.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
        assert_eq!(split_saved(&words).err(), Some(DecodeError::UnknownHashTag(SEED_TRAILER_TAG)));

        // A trailer tag with no room for a seed.
        let short: Vec<u8> = SEED_TRAILER_TAG.to_le_bytes().to_vec();
        assert_eq!(decode_saved(&short).unwrap_err(), DecodeError::Truncated);
    }

    fn leaf_proof(flag: u64) -> VadcopFinalProof {
        let mut publics = vec![0u64; expected_n_publics(false)];
        publics[0] = flag;
        let words = proof_size("Poseidon2", false).unwrap().max_bytes() / 8;
        VadcopFinalProof::new(vec![0; words], publics, false, "Poseidon2".into())
    }

    /// Each shape check names itself; only a well-shaped proof reaches the STARK.
    #[test]
    fn verify_names_the_check_that_failed() {
        let vk = [1, 2, 3, 4];
        let err = |p: &VadcopFinalProof| verify(p, &vk).unwrap_err();
        assert_eq!(err(&leaf_proof(IS_VADCOP_FINAL_PROOF)), VerifyError::InvalidProof);
        // A fold declaring a domain other than the key fails before the STARK runs.
        assert_eq!(err(&leaf_proof(0)), VerifyError::AggregateDomainMismatch { declared: [0; 4] });

        let mut wide = leaf_proof(IS_VADCOP_FINAL_PROOF);
        wide.public_values[1 + crate::PROGRAM_VK_LEN + 3] = 1 << 32;
        assert_eq!(err(&wide), VerifyError::OutputNotU32 { index: 3 });
        assert_eq!(err(&leaf_proof(2)), VerifyError::InvalidLeafFlag(2));

        let mut p = leaf_proof(IS_VADCOP_FINAL_PROOF);
        p.public_values[7] = GOLDILOCKS_ORDER;
        assert_eq!(err(&p), VerifyError::NonCanonicalPublic { index: 7 });

        let mut p = leaf_proof(IS_VADCOP_FINAL_PROOF);
        p.public_values.pop();
        assert!(matches!(err(&p), VerifyError::PublicCount { .. }));

        let mut p = leaf_proof(IS_VADCOP_FINAL_PROOF);
        p.proof.pop();
        assert!(matches!(err(&p), VerifyError::ProofLength { .. }));
        p.proof.extend([0, 0]);
        assert!(matches!(err(&p), VerifyError::ProofLength { .. }));

        let mut p = leaf_proof(IS_VADCOP_FINAL_PROOF);
        p.compressed = true;
        p.hash = "sha3".into();
        assert!(matches!(err(&p), VerifyError::UnsupportedStage { .. }));
    }
}
