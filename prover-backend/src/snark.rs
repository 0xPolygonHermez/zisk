//! The protocol of the final SNARK a SNARK proving key is set up for.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{anyhow, Result};
use proofman::FinalSnarkKey;
use zisk_common::SnarkProtocol;

/// The protocol ids a zkey names in its header, snarkjs's (proofman's `SnarkProtocol` ids).
const ZKEY_PLONK: u32 = 2;
const ZKEY_FFLONK: u32 = 10;

/// The protocol of the final SNARK key of `proving_key_snark`, found as the prover finds it.
/// Errors when it has no usable final SNARK key, or one of a protocol ZisK does not prove with.
pub fn snark_protocol_of_key(proving_key_snark: &Path) -> Result<SnarkProtocol> {
    let refuse =
        |found: String| anyhow!("the SNARK proving key {} {found}", proving_key_snark.display());
    match FinalSnarkKey::find(&proving_key_snark.join("final").join("final")) {
        Ok(FinalSnarkKey::Pilfflonk(_)) => Ok(SnarkProtocol::Pilfflonk),
        Ok(FinalSnarkKey::Zkey(zkey)) => match zkey_protocol(&zkey) {
            Ok(ZKEY_PLONK) => Ok(SnarkProtocol::Plonk),
            Ok(ZKEY_FFLONK) => Err(refuse("is set up for FFLONK".to_string())),
            Ok(id) => Err(refuse(format!("names protocol {id} in {}", zkey.display()))),
            Err(e) => Err(refuse(format!("has an unreadable {}: {e}", zkey.display()))),
        },
        Err(e) => Err(refuse(format!("has no usable final SNARK key: {e}"))),
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
    use super::{snark_protocol_of_key, ZKEY_FFLONK, ZKEY_PLONK};
    use std::path::PathBuf;
    use zisk_common::SnarkProtocol;

    /// A `provingKeySnark/` in a fresh directory, removed on drop.
    struct SnarkKeyDir(PathBuf);

    impl SnarkKeyDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("prover_backend_snark_key_{tag}_{}", std::process::id()));
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

    fn refusal(key: &SnarkKeyDir) -> String {
        snark_protocol_of_key(&key.0).unwrap_err().to_string()
    }

    #[test]
    fn a_plonk_zkey_is_a_plonk_key() {
        let key = SnarkKeyDir::with_zkey("plonk", ZKEY_PLONK);
        assert_eq!(snark_protocol_of_key(&key.0).unwrap(), SnarkProtocol::Plonk);
    }

    #[test]
    fn an_fflonk_key_is_refused() {
        let key = SnarkKeyDir::with_zkey("fflonk", ZKEY_FFLONK);
        assert!(refusal(&key).ends_with("is set up for FFLONK"));
    }

    #[test]
    fn a_zkey_of_another_protocol_is_refused() {
        let key = SnarkKeyDir::with_zkey("groth16", 1);
        assert!(refusal(&key).contains("names protocol 1 in"));
    }

    #[test]
    fn a_zkey_that_is_not_one_is_refused() {
        let key = SnarkKeyDir::new("garbage");
        std::fs::write(key.0.join("final").join("final.zkey"), "garbage").unwrap();
        assert!(refusal(&key).contains("has an unreadable"));
    }

    #[test]
    fn a_key_without_a_final_snark_is_refused() {
        let key = SnarkKeyDir::new("empty");
        assert!(refusal(&key).contains("has no usable final SNARK key"));
    }

    /// `ZISK_TEST_PILFFLONK_KEY`: a `provingKeySnark/` set up for pilfflonk.
    #[test]
    #[ignore = "needs a pilfflonk SNARK proving key, in ZISK_TEST_PILFFLONK_KEY"]
    fn a_pilfflonk_key_is_a_pilfflonk_key() {
        let key = PathBuf::from(std::env::var_os("ZISK_TEST_PILFFLONK_KEY").unwrap());
        assert_eq!(snark_protocol_of_key(&key).unwrap(), SnarkProtocol::Pilfflonk);
    }
}
