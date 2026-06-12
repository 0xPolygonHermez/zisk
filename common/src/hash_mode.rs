use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Hashing mode used for the ROM merkle tree and the STARK.
///
/// Different modes may use different merkle-tree parameters and are encoded in
/// the cache/verkey filenames so their artifacts never collide. A verkey is
/// only valid relative to a mode, so the mode travels with the verkey
/// ([`crate::ProgramVK`]) and is compared against a proof's hash family at
/// verify time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum HashMode {
    #[default]
    Poseidon1,
    Poseidon2,
}

impl HashMode {
    /// Merkle-tree arity for this mode. Currently equal across modes, but kept
    /// per-mode so they can diverge without touching call sites.
    pub fn merkle_tree_arity(&self) -> u64 {
        match self {
            HashMode::Poseidon1 => 4,
            HashMode::Poseidon2 => 4,
        }
    }

    /// Trace blowup factor for this mode.
    pub fn blowup_factor(&self) -> u64 {
        match self {
            HashMode::Poseidon1 => 2,
            HashMode::Poseidon2 => 2,
        }
    }

    /// Short, lowercase tag embedded in cache/verkey filenames so the two modes'
    /// artifacts are distinct on disk.
    pub fn file_tag(&self) -> &'static str {
        match self {
            HashMode::Poseidon1 => "poseidon1",
            HashMode::Poseidon2 => "poseidon2",
        }
    }

    /// Canonical capitalized name (`"Poseidon1"`/`"Poseidon2"`), matching the
    /// hash family string carried in proofs and DTOs.
    pub fn as_str(&self) -> &'static str {
        match self {
            HashMode::Poseidon1 => "Poseidon1",
            HashMode::Poseidon2 => "Poseidon2",
        }
    }
}

impl std::str::FromStr for HashMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "poseidon1" => Ok(HashMode::Poseidon1),
            "poseidon2" => Ok(HashMode::Poseidon2),
            other => Err(anyhow::anyhow!("unrecognized HashMode: {other:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HashMode;
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

    #[test]
    fn hash_mode_as_str_roundtrips_through_from_str() {
        for m in [HashMode::Poseidon1, HashMode::Poseidon2] {
            assert_eq!(HashMode::from_str(m.as_str()).unwrap(), m);
        }
    }
}
