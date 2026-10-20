//! Proof-kind selection and labelling shared by the prove / wrap / verify commands.
//!
//! Centralises logic that was previously duplicated across the embedded, remote,
//! and dev command trees, and makes it unit-testable in isolation.

use anyhow::Result;
use zisk_sdk::{Proof, ProofBody, ProofKind};

/// Select the proof kind for a *prove* request.
///
/// Proving defaults to a full STARK proof (`VadcopFinal`); `--plonk` and
/// `--minimal` select a wrapped proof instead. The two flags are mutually
/// exclusive at the clap layer; if both are somehow set, `--plonk` wins so the
/// result stays deterministic.
pub(crate) fn select_prove_kind(plonk: bool, minimal: bool) -> ProofKind {
    if plonk {
        ProofKind::Plonk
    } else if minimal {
        ProofKind::VadcopFinalMinimal
    } else {
        ProofKind::VadcopFinal
    }
}

/// Select the proof kind for a *wrap* request.
///
/// Unlike proving, wrapping has no default: exactly one of `--plonk` /
/// `--minimal` must be supplied, otherwise it is a usage error.
pub(crate) fn select_wrap_kind(plonk: bool, minimal: bool) -> Result<ProofKind> {
    if plonk {
        Ok(ProofKind::Plonk)
    } else if minimal {
        Ok(ProofKind::VadcopFinalMinimal)
    } else {
        anyhow::bail!("Either --plonk or --minimal must be specified.")
    }
}

/// Label for a wrapped proof, used in the `wrap` summary line ("PLONK" / "minimal").
pub(crate) fn wrap_kind_label(kind: ProofKind) -> &'static str {
    match kind {
        ProofKind::Plonk => "PLONK",
        _ => "minimal",
    }
}

/// Proof-family label used by `verify` ("STARK" for VADCOP proofs, "PILFFLONK" or "PLONK" for the
/// SNARK of that protocol). Both SNARKs have the `Plonk` kind, so the body says which.
pub(crate) fn verify_kind_label(proof: &Proof) -> &'static str {
    match (proof.kind(), &proof.body) {
        (ProofKind::VadcopFinal | ProofKind::VadcopFinalMinimal, _) => "STARK",
        (ProofKind::Plonk, ProofBody::Pilfflonk { .. }) => "PILFFLONK",
        (ProofKind::Plonk, _) => "PLONK",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prove_kind_defaults_to_vadcop_final() {
        assert!(matches!(select_prove_kind(false, false), ProofKind::VadcopFinal));
    }

    #[test]
    fn prove_kind_plonk_and_minimal_select_wrapped() {
        assert!(matches!(select_prove_kind(true, false), ProofKind::Plonk));
        assert!(matches!(select_prove_kind(false, true), ProofKind::VadcopFinalMinimal));
    }

    #[test]
    fn prove_kind_plonk_wins_when_both_set() {
        // clap forbids this combination, but the precedence must stay deterministic.
        assert!(matches!(select_prove_kind(true, true), ProofKind::Plonk));
    }

    #[test]
    fn wrap_kind_requires_a_flag() {
        assert!(select_wrap_kind(false, false).is_err());
        assert!(matches!(select_wrap_kind(true, false).unwrap(), ProofKind::Plonk));
        assert!(matches!(select_wrap_kind(false, true).unwrap(), ProofKind::VadcopFinalMinimal));
    }

    #[test]
    fn wrap_label_maps_plonk_else_minimal() {
        assert_eq!(wrap_kind_label(ProofKind::Plonk), "PLONK");
        assert_eq!(wrap_kind_label(ProofKind::VadcopFinalMinimal), "minimal");
        assert_eq!(wrap_kind_label(ProofKind::VadcopFinal), "minimal");
    }

    fn proof_of(body: ProofBody) -> Proof {
        Proof::new(body, zisk_sdk::ProgramVK::new_empty())
    }

    #[test]
    fn verify_label_names_the_protocol_of_the_proof() {
        let vadcop = proof_of(ProofBody::default());
        assert_eq!(verify_kind_label(&vadcop), "STARK");

        let pilfflonk = proof_of(ProofBody::Pilfflonk {
            proof_bytes: vec![],
            vkey_json: "{}".to_string(),
            publics: zisk_sdk::PublicValues::new_empty(),
            publics_full: vec![],
            rootc: vec![],
        });
        assert_eq!(pilfflonk.kind(), ProofKind::Plonk);
        assert_eq!(verify_kind_label(&pilfflonk), "PILFFLONK");
    }
}
