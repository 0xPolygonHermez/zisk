//! End-to-end integrity digests for proof payloads.
//!
//! Proofs travel from the producing worker to the coordinator and then to the
//! aggregating worker over plaintext gRPC, protected only by TCP's 16-bit
//! checksum. A blake3 digest computed by the producing worker and carried
//! alongside the proof lets both the coordinator (on receipt) and the
//! aggregator (before registering the proof) detect corruption introduced on
//! either network hop or while the proof was parked in coordinator memory.

/// Length in bytes of a proof digest (blake3 output).
pub const PROOF_DIGEST_LEN: usize = 32;

/// Computes the integrity digest of a proof payload.
///
/// The digest binds the proof values to their identity so that cross-wired
/// proofs (right bytes, wrong worker/airgroup) are also detected:
///
/// `blake3( airgroup_id LE || worker_idx LE || values.len() LE || values[i] LE ... )`
pub fn proof_digest(airgroup_id: u64, worker_idx: u32, values: &[u64]) -> [u8; PROOF_DIGEST_LEN] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&airgroup_id.to_le_bytes());
    hasher.update(&worker_idx.to_le_bytes());
    hasher.update(&(values.len() as u64).to_le_bytes());
    // Feed values through a chunked buffer instead of one 8-byte update per
    // element — proofs are megabytes of u64s.
    const CHUNK: usize = 8192;
    let mut buf = Vec::with_capacity(8 * CHUNK);
    for chunk in values.chunks(CHUNK) {
        buf.clear();
        for v in chunk {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        hasher.update(&buf);
    }
    *hasher.finalize().as_bytes()
}

/// Outcome of verifying a proof payload against its carried digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofDigestCheck {
    /// Digest present and matching.
    Ok,
    /// Peer sent no digest (older peer version) — verification skipped.
    Missing,
    /// Digest present but not matching — payload corrupted or mislabeled.
    /// Digests are truncated hex, for logging.
    Mismatch { expected: String, computed: String },
}

/// Verifies `values` against the digest carried with the proof.
///
/// An empty `expected` digest means the sender predates integrity digests and
/// yields [`ProofDigestCheck::Missing`]. A digest of the wrong length can never
/// match and yields [`ProofDigestCheck::Mismatch`].
pub fn check_proof_digest(
    airgroup_id: u64,
    worker_idx: u32,
    values: &[u64],
    expected: &[u8],
) -> ProofDigestCheck {
    if expected.is_empty() {
        return ProofDigestCheck::Missing;
    }
    let computed = proof_digest(airgroup_id, worker_idx, values);
    if expected == computed {
        ProofDigestCheck::Ok
    } else {
        ProofDigestCheck::Mismatch { expected: hex_trunc(expected), computed: hex_trunc(&computed) }
    }
}

/// First 8 bytes of a digest as hex — enough to correlate logs.
fn hex_trunc(digest: &[u8]) -> String {
    hex::encode(&digest[..digest.len().min(8)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_deterministic() {
        let values = vec![1u64, 2, 3, u64::MAX];
        assert_eq!(proof_digest(7, 3, &values), proof_digest(7, 3, &values));
    }

    #[test]
    fn digest_known_answer() {
        // Pinned so accidental changes to the digest format are caught.
        let digest = proof_digest(1, 2, &[3, 4]);
        assert_eq!(
            hex::encode(digest),
            "60639abc8dfffc7fcb8369a8cf454f607d6998a0b5cafe0efe86af4a7844891d"
        );
    }

    #[test]
    fn digest_is_sensitive_to_every_input() {
        let values = vec![1u64, 2, 3];
        let base = proof_digest(7, 3, &values);

        let mut flipped = values.clone();
        flipped[1] ^= 1;
        assert_ne!(base, proof_digest(7, 3, &flipped), "value bit flip must change digest");
        assert_ne!(base, proof_digest(8, 3, &values), "airgroup_id must change digest");
        assert_ne!(base, proof_digest(7, 4, &values), "worker_idx must change digest");
        assert_ne!(base, proof_digest(7, 3, &[1, 2, 3, 0]), "appending a value must change digest");
    }

    #[test]
    fn check_reports_missing_ok_and_mismatch() {
        let values = vec![10u64, 20, 30];
        let digest = proof_digest(1, 0, &values);

        assert_eq!(check_proof_digest(1, 0, &values, &[]), ProofDigestCheck::Missing);
        assert_eq!(check_proof_digest(1, 0, &values, &digest), ProofDigestCheck::Ok);

        let mut corrupted = digest;
        corrupted[0] ^= 0xff;
        assert!(matches!(
            check_proof_digest(1, 0, &values, &corrupted),
            ProofDigestCheck::Mismatch { .. }
        ));
        // Wrong-length digest can never match.
        assert!(matches!(
            check_proof_digest(1, 0, &values, &digest[..16]),
            ProofDigestCheck::Mismatch { .. }
        ));
    }
}
