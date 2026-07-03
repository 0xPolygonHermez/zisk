use thiserror::Error;

use crate::manifest::RecurserManifestInputs;
use crate::templates::IS_VADCOP_FINAL_SLOT;

/// Layout of a vadcop_final `public_values` blob:
/// `[is_vadcop_final_proof(1)][program_vk(4)][user_publics(64)]`.
/// See `zisk/common/src/proof.rs` and `recurser/src/templates.rs`.
const PROGRAM_VK_LEN: usize = 4;
/// First index of the programVK in public_values (after the is_vadcop_final slot).
const VK_BASE: usize = IS_VADCOP_FINAL_SLOT + 1;
/// Minimum public_values length: slot 0 flag + 4-limb programVK.
const MIN_PUBLICS_LEN: usize = VK_BASE + PROGRAM_VK_LEN; // = 5

/// Classification derived from `public_values[IS_VADCOP_FINAL_SLOT]`.
/// - `1` → raw ZisK vadcop_final leaf.
/// - `0` → output of a prior recursion round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofOrigin {
    Leaf,
    Aggregated,
}

#[derive(Debug, Error)]
pub enum ProveValidationError {
    #[error(
        "proof_{side}'s public_values length ({got}) is too short (need >= {MIN_PUBLICS_LEN}: \
         1 flag slot + {PROGRAM_VK_LEN}-limb programVK)"
    )]
    PublicsTooShort { side: char, got: usize },

    #[error(
        "proof_{side}'s is_vadcop_final_proof flag at slot {IS_VADCOP_FINAL_SLOT} is {value}; \
         expected 0 (aggregated) or 1 (leaf)"
    )]
    InvalidVadcopFinalFlag { side: char, value: u64 },

    #[error(
        "free_inputs_{side}: {got} values supplied but the circuit expects exactly {expected}. \
         The free-input arrays are laid out positionally in the witness buffer ahead of \
         rootCRecurserAgg, so a wrong length shears every following input"
    )]
    FreeInputsLength { side: char, got: usize, expected: usize },
}

/// Pre-check inputs at the CLI boundary so errors surface as clear messages
/// rather than cryptic constraint violations deep inside proofman.
///
/// Classification uses `public_values[IS_VADCOP_FINAL_SLOT]`:
/// - `1` → `Leaf`  (the free array is free_in, consumed by `NormalizePublics`).
/// - `0` → `Aggregated` (the free array is free_out, fed to `AggregatePublics`).
///
/// The caller passes ONE free array per side (width `n_free`). The leaf/aggregated
/// distinction drives runtime semantics but NOT validation: both origins check the
/// same array against the single `n_free`.
///
/// Free-input rule (per side): the array length must equal `n_free` exactly.
/// The witness (`zkin`) buffer is filled positionally — `proof_a`, `proof_b`,
/// `freeInputsA`, `freeInputsB`, `rootCRecurserAgg` — and the backend appends
/// each free array by its supplied length with no padding, so any length other
/// than `n_free` shifts `rootCRecurserAgg` and shears the witness. Both under-
/// and oversupply are therefore errors.
pub fn validate_prove_inputs(
    manifest_inputs: &RecurserManifestInputs,
    proof_a_publics: &[u64],
    proof_b_publics: &[u64],
    free_a: &[u64],
    free_b: &[u64],
) -> Result<(ProofOrigin, ProofOrigin), ProveValidationError> {
    let origin_a = classify_proof('a', proof_a_publics)?;
    let origin_b = classify_proof('b', proof_b_publics)?;

    let n_free = manifest_inputs.n_free();

    validate_free_inputs('a', free_a, n_free)?;
    validate_free_inputs('b', free_b, n_free)?;

    Ok((origin_a, origin_b))
}

fn classify_proof(side: char, publics: &[u64]) -> Result<ProofOrigin, ProveValidationError> {
    if publics.len() < MIN_PUBLICS_LEN {
        return Err(ProveValidationError::PublicsTooShort { side, got: publics.len() });
    }
    match publics[IS_VADCOP_FINAL_SLOT] {
        1 => Ok(ProofOrigin::Leaf),
        0 => Ok(ProofOrigin::Aggregated),
        v => Err(ProveValidationError::InvalidVadcopFinalFlag { side, value: v }),
    }
}

fn validate_free_inputs(
    side: char,
    free: &[u64],
    n_free: usize,
) -> Result<(), ProveValidationError> {
    // The backend fills the witness buffer positionally with no padding, so the
    // free array must be exactly n_free wide — under- and oversupply both shear
    // the buffer (rootCRecurserAgg follows the two free arrays).
    if free.len() != n_free {
        return Err(ProveValidationError::FreeInputsLength {
            side,
            got: free.len(),
            expected: n_free,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vk_str(values: [u64; 4]) -> [String; 4] {
        values.map(|v| v.to_string())
    }

    /// Build a manifest with the given single free-input width.
    fn manifest(n_free: usize) -> RecurserManifestInputs {
        use crate::templates::NormalizeCircuit;
        let zisk_vk = vk_str([100, 101, 102, 103]);
        let normalize = Some(NormalizeCircuit { body: "// norm".into() });
        RecurserManifestInputs::new(zisk_vk, vec![], normalize.as_ref(), "agg", n_free)
    }

    /// Build `public_values` with the flag at slot 0 and a 4-limb programVK at [1..5).
    fn publics(flag: u64, vk: [u64; 4]) -> Vec<u64> {
        let mut p = vec![flag];
        p.extend(vk);
        p.extend(vec![0u64; 64]);
        p
    }

    fn leaf_publics() -> Vec<u64> {
        publics(1, [1, 2, 3, 4])
    }

    fn agg_publics() -> Vec<u64> {
        publics(0, [9, 9, 9, 9])
    }

    // --- classification ---

    #[test]
    fn leaf_flag_one_classified_as_leaf() {
        let m = manifest(0);
        let p = leaf_publics();
        let (oa, ob) = validate_prove_inputs(&m, &p, &p, &[], &[]).unwrap();
        assert_eq!(oa, ProofOrigin::Leaf);
        assert_eq!(ob, ProofOrigin::Leaf);
    }

    #[test]
    fn aggregated_flag_zero_classified_as_aggregated() {
        let m = manifest(0);
        let p = agg_publics();
        let (oa, ob) = validate_prove_inputs(&m, &p, &p, &[], &[]).unwrap();
        assert_eq!(oa, ProofOrigin::Aggregated);
        assert_eq!(ob, ProofOrigin::Aggregated);
    }

    #[test]
    fn leaf_and_aggregated_mixed() {
        let m = manifest(3);
        let la = leaf_publics();
        let ab = agg_publics();
        let (oa, ob) = validate_prove_inputs(&m, &la, &ab, &[1, 2, 3], &[20, 21, 22]).unwrap();
        assert_eq!(oa, ProofOrigin::Leaf);
        assert_eq!(ob, ProofOrigin::Aggregated);
    }

    #[test]
    fn rejects_invalid_flag_value() {
        let m = manifest(0);
        let bad = publics(42, [1, 2, 3, 4]);
        let p = leaf_publics();
        let err = validate_prove_inputs(&m, &bad, &p, &[], &[]).unwrap_err();
        assert!(
            matches!(err, ProveValidationError::InvalidVadcopFinalFlag { side: 'a', value: 42 }),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_invalid_flag_on_b_side() {
        let m = manifest(0);
        let p = leaf_publics();
        let bad = publics(99, [5, 6, 7, 8]);
        let err = validate_prove_inputs(&m, &p, &bad, &[], &[]).unwrap_err();
        assert!(
            matches!(err, ProveValidationError::InvalidVadcopFinalFlag { side: 'b', value: 99 }),
            "unexpected error: {err}"
        );
    }

    // --- too-short publics ---

    #[test]
    fn rejects_too_short_publics_a() {
        let m = manifest(0);
        let short = vec![1u64, 2]; // only 2 elements; need >= 5
        let p = leaf_publics();
        let err = validate_prove_inputs(&m, &short, &p, &[], &[]).unwrap_err();
        assert!(
            matches!(err, ProveValidationError::PublicsTooShort { side: 'a', got: 2 }),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_too_short_publics_b() {
        let m = manifest(0);
        let p = leaf_publics();
        let short = vec![0u64; 4]; // 4 elements; need >= 5
        let err = validate_prove_inputs(&m, &p, &short, &[], &[]).unwrap_err();
        assert!(
            matches!(err, ProveValidationError::PublicsTooShort { side: 'b', got: 4 }),
            "unexpected error: {err}"
        );
    }

    // --- single free array respected against n_free regardless of origin ---
    //
    // The old model rejected "normalize free inputs on an aggregated proof".
    // With the unified single array there is no such rule: an aggregated side
    // supplies its free_out through the SAME array, checked against the same
    // n_free. So an exactly-n_free-wide array is accepted for an aggregated side.
    #[test]
    fn accepts_free_inputs_on_aggregated_proof() {
        let m = manifest(3);
        let leaf = leaf_publics();
        let agg = agg_publics();
        // side b is aggregated; an exactly-n_free-wide free array is fine.
        let (oa, ob) = validate_prove_inputs(&m, &leaf, &agg, &[1, 2, 3], &[97, 98, 99]).unwrap();
        assert_eq!(oa, ProofOrigin::Leaf);
        assert_eq!(ob, ProofOrigin::Aggregated);
    }

    // --- oversupply (against the single n_free) ---

    #[test]
    fn rejects_oversupplied_free_inputs_a() {
        let m = manifest(2);
        let p = leaf_publics();
        // 3 > n_free=2
        let err = validate_prove_inputs(&m, &p, &p, &[1, 2, 3], &[]).unwrap_err();
        assert!(
            matches!(
                err,
                ProveValidationError::FreeInputsLength { side: 'a', got: 3, expected: 2 }
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_oversupplied_free_inputs_b() {
        let m = manifest(3);
        let p = agg_publics();
        // side a is exactly n_free=3; side b oversupplies (4 > 3).
        let err = validate_prove_inputs(&m, &p, &p, &[1, 2, 3], &[1, 2, 3, 4]).unwrap_err();
        assert!(
            matches!(
                err,
                ProveValidationError::FreeInputsLength { side: 'b', got: 4, expected: 3 }
            ),
            "unexpected error: {err}"
        );
    }

    // --- undersupply is an error (no padding; a short array shears the witness) ---

    #[test]
    fn accepts_exact_width_free_inputs() {
        let m = manifest(5);
        let p = leaf_publics();
        // Exactly n_free=5 on both sides.
        validate_prove_inputs(&m, &p, &p, &[1, 2, 3, 4, 5], &[10, 11, 12, 13, 14]).unwrap();
    }

    #[test]
    fn rejects_undersupplied_free_inputs_a() {
        let m = manifest(5);
        let p = leaf_publics();
        // 1 < n_free=5 — must be rejected (proofman does not pad).
        let err = validate_prove_inputs(&m, &p, &p, &[1], &[10, 11, 12, 13, 14]).unwrap_err();
        assert!(
            matches!(
                err,
                ProveValidationError::FreeInputsLength { side: 'a', got: 1, expected: 5 }
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_undersupplied_free_inputs_b() {
        let m = manifest(3);
        let p = agg_publics();
        // 2 < n_free=3 on side b.
        let err = validate_prove_inputs(&m, &p, &p, &[1, 2, 3], &[10, 11]).unwrap_err();
        assert!(
            matches!(
                err,
                ProveValidationError::FreeInputsLength { side: 'b', got: 2, expected: 3 }
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn accepts_empty_free_inputs_when_circuit_needs_none() {
        let m = manifest(0);
        let p = leaf_publics();
        validate_prove_inputs(&m, &p, &p, &[], &[]).unwrap();
    }
}
