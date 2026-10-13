//! Per-execution seed.
//!
//! Every execution of a guest runs with a 256-bit seed from which the emulators derive the values
//! returned to the guest by `zkvm_random_u64`. Those values are free witness values: they are not
//! proven, so the seed only matters for reproducing a run. A seed is drawn from the OS CSPRNG for
//! every task unless the host forces one (replays, tests, CI). It is decided once per task, before
//! any emulator runs, because every emulation of the task (parallel emulator threads, MPI ranks,
//! distributed workers) must see the same values.

use crate::error::{CommonError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// The 256-bit seed of one execution.
///
/// Parsed from and displayed as 64 hex characters; [`FromStr`] also accepts a `0x` prefix.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionSeed([u8; ExecutionSeed::LEN]);

impl ExecutionSeed {
    /// Seed length in bytes.
    pub const LEN: usize = 32;

    /// Wraps raw seed bytes.
    pub const fn new(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes)
    }

    /// Draws a fresh seed from the operating system's CSPRNG.
    ///
    /// # Errors
    ///
    /// Returns [`CommonError::Io`] if the OS random source is unavailable.
    pub fn random() -> Result<Self> {
        let mut bytes = [0u8; Self::LEN];
        getrandom::getrandom(&mut bytes)
            .map_err(|e| CommonError::Io(format!("failed to draw a random execution seed: {e}")))?;
        Ok(Self(bytes))
    }

    /// The raw seed bytes.
    pub const fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }
}

impl From<[u8; ExecutionSeed::LEN]> for ExecutionSeed {
    fn from(bytes: [u8; ExecutionSeed::LEN]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<&[u8]> for ExecutionSeed {
    type Error = CommonError;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let bytes: [u8; Self::LEN] = bytes.try_into().map_err(|_| {
            CommonError::Invalid(format!(
                "execution seed must be {} bytes, got {}",
                Self::LEN,
                bytes.len()
            ))
        })?;
        Ok(Self(bytes))
    }
}

impl FromStr for ExecutionSeed {
    type Err = CommonError;

    fn from_str(s: &str) -> Result<Self> {
        let digits = s.strip_prefix("0x").unwrap_or(s);
        if digits.len() != 2 * Self::LEN {
            return Err(CommonError::Invalid(format!(
                "execution seed must be {} hex characters, got {}",
                2 * Self::LEN,
                digits.len()
            )));
        }
        let mut bytes = [0u8; Self::LEN];
        hex::decode_to_slice(digits, &mut bytes)
            .map_err(|e| CommonError::Invalid(format!("invalid execution seed: {e}")))?;
        Ok(Self(bytes))
    }
}

impl fmt::Display for ExecutionSeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl fmt::Debug for ExecutionSeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExecutionSeed({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    fn counting_seed() -> ExecutionSeed {
        ExecutionSeed::new(std::array::from_fn(|i| i as u8))
    }

    #[test]
    fn display_round_trips_through_from_str() {
        let seed = counting_seed();
        assert_eq!(seed.to_string(), HEX);
        assert_eq!(HEX.parse::<ExecutionSeed>().unwrap(), seed);
    }

    #[test]
    fn from_str_accepts_0x_prefix_and_uppercase() {
        let upper = format!("0x{}", HEX.to_uppercase());
        assert_eq!(upper.parse::<ExecutionSeed>().unwrap(), counting_seed());
    }

    #[test]
    fn from_str_rejects_wrong_length_and_non_hex() {
        assert!(HEX[..62].parse::<ExecutionSeed>().is_err());
        assert!(format!("{HEX}00").parse::<ExecutionSeed>().is_err());
        assert!(HEX.replace('0', "g").parse::<ExecutionSeed>().is_err());
        assert!("".parse::<ExecutionSeed>().is_err());
    }

    #[test]
    fn try_from_slice_checks_length() {
        let seed = counting_seed();
        assert_eq!(ExecutionSeed::try_from(&seed.as_bytes()[..]).unwrap(), seed);
        assert!(ExecutionSeed::try_from(&seed.as_bytes()[..31]).is_err());
    }

    #[test]
    fn random_seeds_differ() {
        assert_ne!(ExecutionSeed::random().unwrap(), ExecutionSeed::random().unwrap());
    }

    #[test]
    fn bincode_encodes_as_raw_bytes() {
        let seed = counting_seed();
        let bytes = bincode::serde::encode_to_vec(seed, bincode::config::standard()).unwrap();
        assert_eq!(bytes, seed.as_bytes());
        let (decoded, _): (ExecutionSeed, usize) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
        assert_eq!(decoded, seed);
    }
}
