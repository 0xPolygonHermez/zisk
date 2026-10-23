//! The protocol of the final SNARK, and the checks that a SNARK proving key is set up for it and
//! that a proof is of it.

use std::path::Path;

use zisk_common::{Proof, ProofBody};
use zisk_prover_backend::snark_protocol_of_key;

use crate::{Result, SdkError};

pub use zisk_common::SnarkProtocol;

/// Errors unless `proof` is a final SNARK proof of `protocol`. For a remote client, which cannot
/// see the worker's SNARK proving key, this checks the proof the worker returns instead.
pub(crate) fn ensure_snark_proof(protocol: SnarkProtocol, proof: &Proof) -> Result<()> {
    let found = match &proof.body {
        ProofBody::Plonk { .. } => "a PLONK proof",
        ProofBody::Pilfflonk { .. } => "a pilfflonk proof",
        ProofBody::Vadcop { .. } => "a STARK proof",
    };
    match (protocol, &proof.body) {
        (SnarkProtocol::Plonk, ProofBody::Plonk { .. })
        | (SnarkProtocol::Pilfflonk, ProofBody::Pilfflonk { .. }) => Ok(()),
        _ => Err(SdkError::UnexpectedResponse(format!(
            "a {} final SNARK was asked for, but the prover returned {found}: its SNARK proving key \
             is set up for another protocol",
            protocol.name()
        ))),
    }
}

/// Errors unless the final SNARK key of `proving_key_snark`, found as the prover finds it, is set
/// up for `protocol`.
pub(crate) fn ensure_snark_key(protocol: SnarkProtocol, proving_key_snark: &Path) -> Result<()> {
    let refuse = |found: String| {
        Err(SdkError::InvalidConfig(format!(
            "a {} final SNARK was asked for, but {found}",
            protocol.name()
        )))
    };
    match snark_protocol_of_key(proving_key_snark) {
        Ok(found) if found == protocol => Ok(()),
        Ok(found) => refuse(format!(
            "the SNARK proving key {} is set up for {}",
            proving_key_snark.display(),
            found.name()
        )),
        Err(e) => refuse(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_snark_key, ensure_snark_proof, SnarkProtocol};
    use crate::SdkError;
    use std::path::PathBuf;
    use zisk_common::{Proof, ProofBody};

    /// A `provingKeySnark/` in a fresh directory, removed on drop. The keys themselves are
    /// `zisk_prover_backend::snark_protocol_of_key`'s tests'.
    struct SnarkKeyDir(PathBuf);

    impl SnarkKeyDir {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("sdk_snark_key_{tag}_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("final")).unwrap();
            Self(dir)
        }

        /// A PLONK `final.zkey`, as far as its section table: its header section names protocol 2.
        fn with_plonk_zkey(tag: &str) -> Self {
            let key = Self::new(tag);
            let mut zkey = b"zkey".to_vec();
            zkey.extend(1u32.to_le_bytes()); // version
            zkey.extend(1u32.to_le_bytes()); // sections
            zkey.extend(1u32.to_le_bytes());
            zkey.extend(4u64.to_le_bytes());
            zkey.extend(2u32.to_le_bytes());
            std::fs::write(key.0.join("final").join("final.zkey"), zkey).unwrap();
            key
        }
    }

    impl Drop for SnarkKeyDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn refusal(protocol: SnarkProtocol, key: &SnarkKeyDir) -> String {
        match ensure_snark_key(protocol, &key.0) {
            Err(SdkError::InvalidConfig(message)) => message,
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn a_plonk_key_is_a_plonk_key_only() {
        let key = SnarkKeyDir::with_plonk_zkey("plonk");
        ensure_snark_key(SnarkProtocol::Plonk, &key.0).unwrap();
        assert!(refusal(SnarkProtocol::Pilfflonk, &key).ends_with("is set up for PLONK"));
    }

    #[test]
    fn a_key_without_a_final_snark_is_refused() {
        let key = SnarkKeyDir::new("empty");
        let message = refusal(SnarkProtocol::Pilfflonk, &key);
        assert!(message.starts_with("a pilfflonk final SNARK was asked for, but"), "{message}");
        assert!(message.contains("has no usable final SNARK key"), "{message}");
    }

    fn pilfflonk_proof() -> Proof {
        Proof {
            body: ProofBody::Pilfflonk {
                proof_bytes: Vec::new(),
                vkey_json: String::new(),
                publics: Default::default(),
                publics_full: Vec::new(),
                rootc: Vec::new(),
            },
            ..Default::default()
        }
    }

    #[test]
    fn a_proof_is_of_its_protocol_only() {
        ensure_snark_proof(SnarkProtocol::Pilfflonk, &pilfflonk_proof()).unwrap();
        match ensure_snark_proof(SnarkProtocol::Plonk, &pilfflonk_proof()) {
            Err(SdkError::UnexpectedResponse(message)) => {
                assert!(message.contains("returned a pilfflonk proof"), "{message}")
            }
            other => panic!("expected UnexpectedResponse, got {other:?}"),
        }
        let stark = Proof::default();
        assert!(ensure_snark_proof(SnarkProtocol::Pilfflonk, &stark).is_err());
    }
}
