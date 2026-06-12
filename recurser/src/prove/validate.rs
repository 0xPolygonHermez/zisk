use thiserror::Error;

use crate::manifest::RecurserManifestInputs;

/// Layout of a vadcop_final `public_values` blob:
/// `[program_vk(4)][user_publics(64)]`. See `zisk/common/src/proof.rs`.
const PROGRAM_VK_LEN: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramVkOrigin {
    /// `programVK` matches `manifest.inputs.program_vks[idx]`.
    RegisteredProgram(usize),
    /// `programVK` matches `root_c_recurser_agg`.
    PriorAggregation,
}

#[derive(Debug, Error)]
pub enum ProveValidationError {
    #[error(
        "proof_{side}'s public_values length ({got}) is too short to contain a {PROGRAM_VK_LEN}-limb programVK"
    )]
    PublicsTooShort { side: char, got: usize },

    #[error(
        "proof_{side}'s programVK {vk:?} is neither in the registered-program allowlist \
         ({n_programs} entries) nor equal to root_c_recurser_agg {root_c:?}. \
         Check --recurser-id, --root-c-recurser-agg, and the program ELFs registered at setup."
    )]
    UnregisteredProgramVk {
        side: char,
        vk: [u64; PROGRAM_VK_LEN],
        root_c: [u64; PROGRAM_VK_LEN],
        n_programs: usize,
    },

    #[error(
        "free_inputs_{side} has {got} entries, but this recurser's normalization circuits \
         consume at most {expected}"
    )]
    FreeInputsLength { side: char, got: usize, expected: usize },

    #[error("manifest field {field} is not a valid u64 ({value:?}: {source})")]
    ManifestParse { field: &'static str, value: String, source: std::num::ParseIntError },
}

/// Pre-check inputs at the CLI boundary so errors surface as clear messages
/// rather than cryptic constraint violations deep inside proofman.
pub fn validate_prove_inputs(
    manifest_inputs: &RecurserManifestInputs,
    proof_a_publics: &[u64],
    proof_b_publics: &[u64],
    free_inputs_a: &[u64],
    free_inputs_b: &[u64],
    root_c_recurser_agg: &[u64; PROGRAM_VK_LEN],
) -> Result<(ProgramVkOrigin, ProgramVkOrigin), ProveValidationError> {
    // Undersupply is fine — the prove path zero-pads each side to the
    // circuit's fixed array size; only oversupply is a caller error.
    let n_free_inputs = manifest_inputs.n_free_inputs();
    for (side, inputs) in [('a', free_inputs_a), ('b', free_inputs_b)] {
        if inputs.len() > n_free_inputs {
            return Err(ProveValidationError::FreeInputsLength {
                side,
                got: inputs.len(),
                expected: n_free_inputs,
            });
        }
    }

    let allowlist = parse_program_vks(&manifest_inputs.program_vks)?;
    let origin_a = classify_proof('a', proof_a_publics, &allowlist, root_c_recurser_agg)?;
    let origin_b = classify_proof('b', proof_b_publics, &allowlist, root_c_recurser_agg)?;
    Ok((origin_a, origin_b))
}

fn classify_proof(
    side: char,
    publics: &[u64],
    allowlist: &[[u64; PROGRAM_VK_LEN]],
    root_c_recurser_agg: &[u64; PROGRAM_VK_LEN],
) -> Result<ProgramVkOrigin, ProveValidationError> {
    let vk = extract_program_vk(side, publics)?;

    if let Some(idx) = allowlist.iter().position(|entry| entry == &vk) {
        return Ok(ProgramVkOrigin::RegisteredProgram(idx));
    }
    if &vk == root_c_recurser_agg {
        return Ok(ProgramVkOrigin::PriorAggregation);
    }

    Err(ProveValidationError::UnregisteredProgramVk {
        side,
        vk,
        root_c: *root_c_recurser_agg,
        n_programs: allowlist.len(),
    })
}

fn extract_program_vk(
    side: char,
    publics: &[u64],
) -> Result<[u64; PROGRAM_VK_LEN], ProveValidationError> {
    if publics.len() < PROGRAM_VK_LEN {
        return Err(ProveValidationError::PublicsTooShort { side, got: publics.len() });
    }
    let mut vk = [0u64; PROGRAM_VK_LEN];
    vk.copy_from_slice(&publics[..PROGRAM_VK_LEN]);
    Ok(vk)
}

fn parse_program_vks(
    program_vks: &[[String; PROGRAM_VK_LEN]],
) -> Result<Vec<[u64; PROGRAM_VK_LEN]>, ProveValidationError> {
    let mut out = Vec::with_capacity(program_vks.len());
    for vk in program_vks {
        let mut limbs = [0u64; PROGRAM_VK_LEN];
        for (i, limb) in vk.iter().enumerate() {
            limbs[i] =
                limb.parse::<u64>().map_err(|source| ProveValidationError::ManifestParse {
                    field: "inputs.program_vks[][]",
                    value: limb.clone(),
                    source,
                })?;
        }
        out.push(limbs);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vk_str(values: [u64; 4]) -> [String; 4] {
        values.map(|v| v.to_string())
    }

    fn manifest(n_free_inputs: usize, program_vks: Vec<[u64; 4]>) -> RecurserManifestInputs {
        let zisk_vk = vk_str([100, 101, 102, 103]);
        let vks: Vec<[String; 4]> = program_vks.into_iter().map(vk_str).collect();
        let groups = if n_free_inputs > 0 {
            vec![crate::templates::NormalizeGroup {
                member_indices: vec![0],
                body: "norm".to_string(),
                n_free_inputs,
            }]
        } else {
            vec![]
        };
        RecurserManifestInputs::new(zisk_vk, vks, &groups, "a")
    }

    fn publics_with_program_vk(vk: [u64; 4]) -> Vec<u64> {
        let mut out = vk.to_vec();
        out.extend(std::iter::repeat(0).take(64));
        out
    }

    #[test]
    fn accepts_two_leaf_proofs() {
        let m = manifest(0, vec![[1, 2, 3, 4], [5, 6, 7, 8]]);
        let a = publics_with_program_vk([1, 2, 3, 4]);
        let b = publics_with_program_vk([5, 6, 7, 8]);

        let (oa, ob) = validate_prove_inputs(&m, &a, &b, &[], &[], &[0; 4]).unwrap();
        assert_eq!(oa, ProgramVkOrigin::RegisteredProgram(0));
        assert_eq!(ob, ProgramVkOrigin::RegisteredProgram(1));
    }

    #[test]
    fn accepts_aggregated_proof_when_program_vk_matches_root_c() {
        let m = manifest(0, vec![[1, 2, 3, 4]]);
        let root_c = [9, 9, 9, 9];
        let a = publics_with_program_vk([1, 2, 3, 4]);
        let b = publics_with_program_vk(root_c);

        let (oa, ob) = validate_prove_inputs(&m, &a, &b, &[], &[], &root_c).unwrap();
        assert_eq!(oa, ProgramVkOrigin::RegisteredProgram(0));
        assert_eq!(ob, ProgramVkOrigin::PriorAggregation);
    }

    #[test]
    fn rejects_unregistered_program_vk() {
        let m = manifest(0, vec![[1, 2, 3, 4]]);
        let a = publics_with_program_vk([1, 2, 3, 4]);
        let b = publics_with_program_vk([42, 42, 42, 42]);

        let err = validate_prove_inputs(&m, &a, &b, &[], &[], &[0; 4]).unwrap_err();
        assert!(matches!(err, ProveValidationError::UnregisteredProgramVk { side: 'b', .. }));
    }

    #[test]
    fn accepts_undersupplied_free_inputs() {
        let m = manifest(3, vec![[1, 2, 3, 4]]);
        let a = publics_with_program_vk([1, 2, 3, 4]);
        let b = publics_with_program_vk([1, 2, 3, 4]);

        // Fewer than n_free_inputs is fine (the prove path zero-pads).
        validate_prove_inputs(&m, &a, &b, &[1, 2], &[], &[0; 4]).unwrap();
    }

    #[test]
    fn rejects_oversupplied_free_inputs() {
        let m = manifest(3, vec![[1, 2, 3, 4]]);
        let a = publics_with_program_vk([1, 2, 3, 4]);
        let b = publics_with_program_vk([1, 2, 3, 4]);

        let err =
            validate_prove_inputs(&m, &a, &b, &[1, 2, 3, 4], &[1, 2, 3], &[0; 4]).unwrap_err();
        assert!(matches!(
            err,
            ProveValidationError::FreeInputsLength { side: 'a', got: 4, expected: 3 }
        ));
    }

    #[test]
    fn rejects_too_short_publics() {
        let m = manifest(0, vec![[1, 2, 3, 4]]);
        let a = vec![1, 2];
        let b = publics_with_program_vk([1, 2, 3, 4]);

        let err = validate_prove_inputs(&m, &a, &b, &[], &[], &[0; 4]).unwrap_err();
        assert!(matches!(err, ProveValidationError::PublicsTooShort { side: 'a', got: 2 }));
    }
}
