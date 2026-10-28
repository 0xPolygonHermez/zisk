//! Proof-kind selection and labelling shared by the prove / wrap / verify commands.
//!
//! Centralises logic that was previously duplicated across the embedded, remote,
//! and dev command trees, and makes it unit-testable in isolation.

use std::path::PathBuf;

use anyhow::Result;
use zisk_common::ZiskPaths;
use zisk_sdk::{Proof, ProofBody, ProofKind, SnarkProtocol};

/// The final SNARK flags of the prove and wrap commands. The SNARK proving key (the workers', for a
/// remote command) is set up for one protocol, which the flag must name: it checks the key, or the
/// proof a worker returns, and does not change the proof.
#[derive(clap::Args, Debug, Clone, Copy, Default)]
pub(crate) struct SnarkArgs {
    /// PLONK SNARK proof, of a SNARK proving key set up for PLONK (`--final-snark plonk`, a
    /// poseidon key only). Mutually exclusive with --pilfflonk and --minimal
    #[arg(long, conflicts_with_all = ["pilfflonk", "minimal"])]
    pub(crate) plonk: bool,

    /// pilfflonk SNARK proof, of a SNARK proving key set up for pilfflonk (the default of
    /// `--final-snark`, and the only one of a blake3 key): the release's, whose contracts are in
    /// `zisk-contracts/<family>/`. Mutually exclusive with --plonk and --minimal
    #[arg(long, conflicts_with_all = ["plonk", "minimal"])]
    pub(crate) pilfflonk: bool,
}

impl SnarkArgs {
    /// The protocol of the final SNARK asked for, if any. clap forbids both flags; if both are
    /// somehow set, `--plonk` wins so the result stays deterministic.
    pub(crate) fn protocol(&self) -> Option<SnarkProtocol> {
        if self.plonk {
            Some(SnarkProtocol::Plonk)
        } else if self.pilfflonk {
            Some(SnarkProtocol::Pilfflonk)
        } else {
            None
        }
    }
}

/// Errors unless the SNARK proving key (`proving_key_snark`, or the default one) is set up for
/// `protocol`. For the dev commands, which drive the backend directly rather than through the SDK.
pub(crate) fn ensure_snark_key(
    protocol: SnarkProtocol,
    proving_key_snark: Option<&PathBuf>,
) -> Result<()> {
    zisk_prover_backend::ensure_snark_key(
        protocol,
        &ZiskPaths::get_proving_key_snark(proving_key_snark),
    )
}

/// Select the proof kind for a *prove* request.
///
/// Proving defaults to a full STARK proof (`VadcopFinal`); a final SNARK (`--plonk`,
/// `--pilfflonk`) or `--minimal` selects a wrapped proof instead. The flags are mutually exclusive
/// at the clap layer; if both are somehow set, the SNARK wins so the result stays deterministic.
pub(crate) fn select_prove_kind(snark: Option<SnarkProtocol>, minimal: bool) -> ProofKind {
    if snark.is_some() {
        ProofKind::Snark
    } else if minimal {
        ProofKind::VadcopFinalMinimal
    } else {
        ProofKind::VadcopFinal
    }
}

/// Select the proof kind for a *wrap* request.
///
/// Unlike proving, wrapping has no default: exactly one of `--plonk` / `--pilfflonk` /
/// `--minimal` must be supplied, otherwise it is a usage error.
pub(crate) fn select_wrap_kind(snark: Option<SnarkProtocol>, minimal: bool) -> Result<ProofKind> {
    if snark.is_some() {
        Ok(ProofKind::Snark)
    } else if minimal {
        Ok(ProofKind::VadcopFinalMinimal)
    } else {
        anyhow::bail!("Either --plonk, --pilfflonk or --minimal must be specified.")
    }
}

/// Label for a wrapped proof, used in the `wrap` summary line ("SNARK" / "minimal").
pub(crate) fn wrap_kind_label(kind: ProofKind) -> &'static str {
    match kind {
        ProofKind::Snark => "SNARK",
        _ => "minimal",
    }
}

/// Proof-family label used by `verify` ("STARK" for VADCOP proofs, "PILFFLONK" or "PLONK" for the
/// SNARK of that protocol). Both SNARKs have the `Snark` kind, so the body says which.
pub(crate) fn verify_kind_label(proof: &Proof) -> &'static str {
    match (proof.kind(), &proof.body) {
        (ProofKind::VadcopFinal | ProofKind::VadcopFinalMinimal, _) => "STARK",
        (ProofKind::Snark, ProofBody::Pilfflonk { .. }) => "PILFFLONK",
        (ProofKind::Snark, _) => "PLONK",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLONK: Option<SnarkProtocol> = Some(SnarkProtocol::Plonk);
    const PILFFLONK: Option<SnarkProtocol> = Some(SnarkProtocol::Pilfflonk);

    #[test]
    fn snark_flags_name_their_protocol() {
        let args = |plonk, pilfflonk| SnarkArgs { plonk, pilfflonk };
        assert_eq!(args(false, false).protocol(), None);
        assert_eq!(args(true, false).protocol(), PLONK);
        assert_eq!(args(false, true).protocol(), PILFFLONK);
        // clap forbids this combination, but the precedence must stay deterministic.
        assert_eq!(args(true, true).protocol(), PLONK);
    }

    #[test]
    fn prove_kind_defaults_to_vadcop_final() {
        assert!(matches!(select_prove_kind(None, false), ProofKind::VadcopFinal));
    }

    #[test]
    fn prove_kind_snark_and_minimal_select_wrapped() {
        assert!(matches!(select_prove_kind(PLONK, false), ProofKind::Snark));
        assert!(matches!(select_prove_kind(PILFFLONK, false), ProofKind::Snark));
        assert!(matches!(select_prove_kind(None, true), ProofKind::VadcopFinalMinimal));
    }

    #[test]
    fn prove_kind_snark_wins_when_both_set() {
        // clap forbids this combination, but the precedence must stay deterministic.
        assert!(matches!(select_prove_kind(PILFFLONK, true), ProofKind::Snark));
    }

    #[test]
    fn wrap_kind_requires_a_flag() {
        assert!(select_wrap_kind(None, false).is_err());
        assert!(matches!(select_wrap_kind(PLONK, false).unwrap(), ProofKind::Snark));
        assert!(matches!(select_wrap_kind(PILFFLONK, false).unwrap(), ProofKind::Snark));
        assert!(matches!(select_wrap_kind(None, true).unwrap(), ProofKind::VadcopFinalMinimal));
    }

    #[test]
    fn wrap_label_maps_snark_else_minimal() {
        assert_eq!(wrap_kind_label(ProofKind::Snark), "SNARK");
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
        assert_eq!(pilfflonk.kind(), ProofKind::Snark);
        assert_eq!(verify_kind_label(&pilfflonk), "PILFFLONK");
    }
}
