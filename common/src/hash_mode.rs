use crate::error::{CommonError, Result};
use crate::paths::ZiskPaths;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Hashing mode used for the ROM merkle tree and the STARK.
///
/// Different modes may use different merkle-tree parameters and are encoded in
/// the cache/verkey filenames so their artifacts never collide. A verkey is
/// only valid relative to a mode, so the mode travels with the verkey
/// ([`crate::ProgramVK`]) and is compared against a proof's hash family at
/// verify time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum HashMode {
    /// Poseidon1 hashing.
    Poseidon1,
    /// Poseidon2 hashing.
    Poseidon2,
    /// Blake hashing
    // The default tracks proofman's `hash_family::DEFAULT_HASH_ID`; a test below pins the two
    // together, since an enum variant cannot be derived from that const.
    #[default]
    Blake3,
}

impl HashMode {
    /// Merkle-tree arity for this mode. Currently equal across modes, but kept
    /// per-mode so they can diverge without touching call sites.
    pub fn merkle_tree_arity(&self) -> u64 {
        match self {
            HashMode::Poseidon1 => 4,
            HashMode::Poseidon2 => 4,
            HashMode::Blake3 => 2,
        }
    }

    /// Trace blowup factor for this mode.
    pub fn blowup_factor(&self) -> u64 {
        match self {
            HashMode::Poseidon1 => 2,
            HashMode::Poseidon2 => 2,
            HashMode::Blake3 => 2,
        }
    }

    /// Whether this mode's proving key can carry a final BN128 SNARK stage.
    ///
    /// The wrap recurses the vadcop_final proof into a circom circuit over BN128, and only the
    /// poseidon families have that path built (proofman's `recursivef` verifier and its circom
    /// templates). `setup-snark` and the wrap commands refuse a Blake3 key.
    pub fn supports_snark(&self) -> bool {
        !matches!(self, HashMode::Blake3)
    }

    /// Short, lowercase tag embedded in cache/verkey filenames so the two modes'
    /// artifacts are distinct on disk.
    pub fn file_tag(&self) -> &'static str {
        match self {
            HashMode::Poseidon1 => "poseidon1",
            HashMode::Poseidon2 => "poseidon2",
            HashMode::Blake3 => "blake3",
        }
    }

    /// Canonical name (`"Poseidon1"`/`"Poseidon2"`/`"blake3"`), matching the
    /// hash family string carried in proofs and DTOs.
    pub fn as_str(&self) -> &'static str {
        match self {
            HashMode::Poseidon1 => "Poseidon1",
            HashMode::Poseidon2 => "Poseidon2",
            HashMode::Blake3 => "blake3",
        }
    }

    /// Hash family a proving key was built with, from its `pilout.globalInfo.json`.
    ///
    /// Never falls back to [`Self::default`]: a verkey is only valid relative to a
    /// mode, so guessing one is the bug this exists to prevent. An unreadable key,
    /// or one with no `hash`, is an error.
    pub fn from_proving_key(proving_key: &Path) -> Result<Self> {
        let path = proving_key.join("pilout.globalInfo.json");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| CommonError::Io(format!("failed to read {}: {e}", path.display())))?;
        let global_info: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            CommonError::Deserialization(format!("failed to parse {}: {e}", path.display()))
        })?;
        global_info
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CommonError::Invalid(format!("no 'hash' in {}", path.display())))?
            .parse()
    }

    /// [`Self::from_proving_key`] for the key `ZiskPaths::global()` resolves to.
    pub fn local() -> Result<Self> {
        Self::from_proving_key(&ZiskPaths::global().proving_key)
    }
}

impl std::str::FromStr for HashMode {
    type Err = CommonError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "poseidon1" => Ok(HashMode::Poseidon1),
            "poseidon2" => Ok(HashMode::Poseidon2),
            "blake3" => Ok(HashMode::Blake3),
            other => Err(CommonError::Invalid(format!("unrecognized HashMode: {other:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HashMode;
    use std::path::Path;
    use std::str::FromStr;

    #[test]
    fn hash_mode_from_str_roundtrip() {
        assert_eq!(HashMode::from_str("Poseidon1").unwrap(), HashMode::Poseidon1);
        assert_eq!(HashMode::from_str("Poseidon2").unwrap(), HashMode::Poseidon2);
    }

    #[test]
    fn hash_mode_from_str_case_insensitive() {
        assert_eq!(HashMode::from_str("poseidon1").unwrap(), HashMode::Poseidon1);
        assert_eq!(HashMode::from_str("POSEIDON2").unwrap(), HashMode::Poseidon2);
    }

    #[test]
    fn hash_mode_from_str_rejects_garbage() {
        assert!(HashMode::from_str("poseidon3").is_err());
        assert!(HashMode::from_str("").is_err());
    }

    /// Writes `contents` (when `Some`) as a proving key's globalInfo.json, then runs
    /// `f` against the key dir.
    fn with_proving_key<T>(contents: Option<&str>, f: impl FnOnce(&Path) -> T) -> T {
        let dir = std::env::temp_dir().join(format!(
            "zisk-hashmode-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        if let Some(c) = contents {
            std::fs::write(dir.join("pilout.globalInfo.json"), c).unwrap();
        }
        let out = f(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        out
    }

    #[test]
    fn from_proving_key_reads_the_hash_field() {
        with_proving_key(Some(r#"{"name":"zisk","hash":"blake3"}"#), |d| {
            assert_eq!(HashMode::from_proving_key(d).unwrap(), HashMode::Blake3);
        });
    }

    /// The whole point of this function: never guess a mode. A missing key, a missing
    /// `hash`, or an unrecognized one must fail rather than fall back to Poseidon1,
    /// which would produce a verkey silently invalid against that key's proofs.
    #[test]
    fn from_proving_key_never_falls_back_to_the_default() {
        with_proving_key(None, |d| assert!(HashMode::from_proving_key(d).is_err()));
        with_proving_key(Some(r#"{"name":"zisk"}"#), |d| {
            assert!(HashMode::from_proving_key(d).is_err())
        });
        with_proving_key(Some(r#"{"hash":"poseidon3"}"#), |d| {
            assert!(HashMode::from_proving_key(d).is_err())
        });
        with_proving_key(Some("not json"), |d| assert!(HashMode::from_proving_key(d).is_err()));
    }

    /// The BN128 wrap is poseidon-only; this must track proofman's `hash_family::supports_snark`.
    #[test]
    fn only_the_poseidon_modes_support_a_snark() {
        assert!(!HashMode::Blake3.supports_snark());
        assert!(HashMode::Poseidon1.supports_snark());
        assert!(HashMode::Poseidon2.supports_snark());
    }

    /// One default, defined in proofman. This enum cannot derive its `#[default]` from that const,
    /// so the pin is a test: if proofman's default moves, move `#[default]` with it.
    #[test]
    fn the_default_mode_is_proofmans_default_family() {
        assert_eq!(HashMode::default().as_str(), proofman_common::hash_family::DEFAULT_HASH_ID);
    }

    /// Every mode this enum knows must be a family proofman knows, and vice versa.
    #[test]
    fn the_modes_match_proofmans_family_list() {
        let mut mine: Vec<&str> = [HashMode::Poseidon1, HashMode::Poseidon2, HashMode::Blake3]
            .iter()
            .map(|m| m.as_str())
            .collect();
        let mut theirs: Vec<&str> = proofman_common::hash_family::FAMILIES.to_vec();
        mine.sort_unstable();
        theirs.sort_unstable();
        assert_eq!(mine, theirs);
    }

    #[test]
    fn hash_mode_as_str_roundtrips_through_from_str() {
        for m in [HashMode::Poseidon1, HashMode::Poseidon2, HashMode::Blake3] {
            assert_eq!(HashMode::from_str(m.as_str()).unwrap(), m);
        }
    }
}
