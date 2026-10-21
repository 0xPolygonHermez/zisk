//! The protocol of the final SNARK, and the checks that a SNARK proving key is set up for it and
//! that a proof is of it.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use proofman::FinalSnarkKey;
use zisk_common::{Proof, ProofBody};

use crate::{Result, SdkError};

/// The protocol of a final SNARK proof. A SNARK proving key is set up for one
/// (`setup-snark --final-snark`), so choosing it checks the key rather than changing the proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnarkProtocol {
    /// PLONK, of a `final.zkey`: a poseidon key only. The one the Solidity verifier takes.
    Plonk,
    /// pilfflonk, of a `provingKey/`: the default of `setup-snark`, and the only one of a blake3
    /// key.
    Pilfflonk,
}

impl SnarkProtocol {
    fn name(self) -> &'static str {
        match self {
            SnarkProtocol::Plonk => "PLONK",
            SnarkProtocol::Pilfflonk => "pilfflonk",
        }
    }
}

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

/// The protocol ids a zkey names in its header, snarkjs's (proofman's `SnarkProtocol` ids).
const ZKEY_PLONK: u32 = 2;
const ZKEY_FFLONK: u32 = 10;

/// Errors unless the final SNARK key of `proving_key_snark`, found as the prover finds it, is set
/// up for `protocol`.
pub(crate) fn ensure_snark_key(protocol: SnarkProtocol, proving_key_snark: &Path) -> Result<()> {
    let refuse = |found: String| {
        Err(SdkError::InvalidConfig(format!(
            "a {} final SNARK was asked for, but the SNARK proving key {} {found}",
            protocol.name(),
            proving_key_snark.display()
        )))
    };
    let found = match FinalSnarkKey::find(&proving_key_snark.join("final").join("final")) {
        Ok(FinalSnarkKey::Pilfflonk(_)) => SnarkProtocol::Pilfflonk,
        Ok(FinalSnarkKey::Zkey(zkey)) => match zkey_protocol(&zkey) {
            Ok(ZKEY_PLONK) => SnarkProtocol::Plonk,
            Ok(ZKEY_FFLONK) => return refuse("is set up for FFLONK".to_string()),
            Ok(id) => return refuse(format!("names protocol {id} in {}", zkey.display())),
            Err(e) => return refuse(format!("has an unreadable {}: {e}", zkey.display())),
        },
        Err(e) => return refuse(format!("has no usable final SNARK key: {e}")),
    };
    if found == protocol {
        Ok(())
    } else {
        refuse(format!("is set up for {}", found.name()))
    }
}

/// The protocol id of a zkey: the first `u32` of its header section (1), which need not be the
/// first section. Reads the section table only, not the key.
fn zkey_protocol(zkey: &Path) -> io::Result<u32> {
    let mut file = File::open(zkey)?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    if &magic != b"zkey" {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not a zkey"));
    }
    let _version = read_u32(&mut file)?;
    let sections = read_u32(&mut file)?;
    for _ in 0..sections {
        let id = read_u32(&mut file)?;
        let size = read_u64(&mut file)?;
        if id == 1 {
            return read_u32(&mut file);
        }
        file.seek(SeekFrom::Current(i64::try_from(size).map_err(io::Error::other)?))?;
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "no header section"))
}

fn read_u32(file: &mut File) -> io::Result<u32> {
    let mut bytes = [0u8; 4];
    file.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(file: &mut File) -> io::Result<u64> {
    let mut bytes = [0u8; 8];
    file.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::{ensure_snark_key, ensure_snark_proof, SnarkProtocol, ZKEY_FFLONK, ZKEY_PLONK};
    use crate::SdkError;
    use std::path::PathBuf;
    use zisk_common::{Proof, ProofBody};

    /// A `provingKeySnark/` in a fresh directory, removed on drop.
    struct SnarkKeyDir(PathBuf);

    impl SnarkKeyDir {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("sdk_snark_key_{tag}_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("final")).unwrap();
            Self(dir)
        }

        /// A `final.zkey` of `protocol`, as far as its section table: an empty section 3 before
        /// the header section, as a PLONK zkey has.
        fn with_zkey(tag: &str, protocol: u32) -> Self {
            let key = Self::new(tag);
            let mut zkey = b"zkey".to_vec();
            zkey.extend(1u32.to_le_bytes()); // version
            zkey.extend(2u32.to_le_bytes()); // sections
            zkey.extend(3u32.to_le_bytes());
            zkey.extend(0u64.to_le_bytes());
            zkey.extend(1u32.to_le_bytes());
            zkey.extend(4u64.to_le_bytes());
            zkey.extend(protocol.to_le_bytes());
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
        let key = SnarkKeyDir::with_zkey("plonk", ZKEY_PLONK);
        ensure_snark_key(SnarkProtocol::Plonk, &key.0).unwrap();
        assert!(refusal(SnarkProtocol::Pilfflonk, &key).ends_with("is set up for PLONK"));
    }

    #[test]
    fn an_fflonk_key_is_refused() {
        let key = SnarkKeyDir::with_zkey("fflonk", ZKEY_FFLONK);
        assert!(refusal(SnarkProtocol::Plonk, &key).ends_with("is set up for FFLONK"));
        assert!(refusal(SnarkProtocol::Pilfflonk, &key).ends_with("is set up for FFLONK"));
    }

    #[test]
    fn a_zkey_that_is_not_one_is_refused() {
        let key = SnarkKeyDir::new("garbage");
        std::fs::write(key.0.join("final").join("final.zkey"), "garbage").unwrap();
        assert!(refusal(SnarkProtocol::Plonk, &key).contains("has an unreadable"));
    }

    #[test]
    fn a_key_without_a_final_snark_is_refused() {
        let key = SnarkKeyDir::new("empty");
        assert!(refusal(SnarkProtocol::Plonk, &key).contains("has no usable final SNARK key"));
        assert!(refusal(SnarkProtocol::Pilfflonk, &key).contains("has no usable final SNARK key"));
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

    /// `ZISK_TEST_PILFFLONK_KEY`: a `provingKeySnark/` set up for pilfflonk.
    #[test]
    #[ignore = "needs a pilfflonk SNARK proving key, in ZISK_TEST_PILFFLONK_KEY"]
    fn a_pilfflonk_key_is_a_pilfflonk_key_only() {
        let key = PathBuf::from(std::env::var_os("ZISK_TEST_PILFFLONK_KEY").unwrap());
        ensure_snark_key(SnarkProtocol::Pilfflonk, &key).unwrap();
        assert!(ensure_snark_key(SnarkProtocol::Plonk, &key).is_err());
    }
}
