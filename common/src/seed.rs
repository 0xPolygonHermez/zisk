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
/// Displayed as 64 hex characters. [`FromStr`] takes 1 to 64 hex characters, optionally `0x`
/// prefixed, and left-pads shorter ones with zeros, so `10` is the seed `00…0010`.
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

    /// The seed as the emulators pass it to the guest: its bytes in order, as 4 little-endian
    /// u64 words (the `zkvm_random_u64` fcall result and the ASM control-input slots).
    pub fn to_le_words(&self) -> [u64; Self::LEN / 8] {
        std::array::from_fn(|i| u64::from_le_bytes(self.0[8 * i..8 * i + 8].try_into().unwrap()))
    }
}

/// `--seed` / `ZISK_SEED`, for every command that runs a guest (cargo-zisk, zisk-prove-client).
#[derive(clap::Args, Debug, Clone, Default)]
pub struct SeedArgs {
    /// Force the execution seed (up to 64 hex characters, zero-padded on the left) instead of
    /// drawing a fresh one, to replay a previous run. Every run reports the seed it used. A remote
    /// coordinator accepts it only if configured to allow client seeds
    #[arg(long, env = "ZISK_SEED", value_name = "HEX")]
    seed: Option<ExecutionSeed>,
}

impl SeedArgs {
    /// The forced seed, if any. Warns when there is one, so a `ZISK_SEED` left in the
    /// environment cannot silently pin every run to the same seed. Call it once per run, after
    /// the logger is up.
    pub fn forced(&self) -> Option<ExecutionSeed> {
        if let Some(seed) = self.seed {
            tracing::warn!("Using a forced execution seed: {seed}");
        }
        self.seed
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
        if digits.is_empty() || digits.len() > 2 * Self::LEN {
            return Err(CommonError::Invalid(format!(
                "execution seed must be 1 to {} hex characters, got {}",
                2 * Self::LEN,
                digits.len()
            )));
        }
        let padded = format!("{digits:0>width$}", width = 2 * Self::LEN);
        let mut bytes = [0u8; Self::LEN];
        hex::decode_to_slice(&padded, &mut bytes)
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
    fn to_le_words_packs_the_bytes_in_order() {
        // The words the guest reads: the same as `ziskemu --random-seed <HEX>`
        assert_eq!(
            counting_seed().to_le_words(),
            [0x0706050403020100, 0x0f0e0d0c0b0a0908, 0x1716151413121110, 0x1f1e1d1c1b1a1918]
        );
    }

    #[test]
    fn from_str_accepts_0x_prefix_and_uppercase() {
        let upper = format!("0x{}", HEX.to_uppercase());
        assert_eq!(upper.parse::<ExecutionSeed>().unwrap(), counting_seed());
    }

    #[test]
    fn from_str_left_pads_short_seeds_with_zeros() {
        let mut ten = [0u8; ExecutionSeed::LEN];
        ten[ExecutionSeed::LEN - 1] = 0x10;
        assert_eq!("10".parse::<ExecutionSeed>().unwrap(), ExecutionSeed::new(ten));
        assert_eq!("0x10".parse::<ExecutionSeed>().unwrap(), ExecutionSeed::new(ten));
        assert_eq!(ExecutionSeed::new(ten).to_string(), format!("{}10", "0".repeat(62)));

        let mut odd = [0u8; ExecutionSeed::LEN];
        odd[ExecutionSeed::LEN - 2..].copy_from_slice(&[0x0a, 0xbc]);
        assert_eq!("abc".parse::<ExecutionSeed>().unwrap(), ExecutionSeed::new(odd));
    }

    #[test]
    fn from_str_rejects_empty_too_long_and_non_hex() {
        assert!("".parse::<ExecutionSeed>().is_err());
        assert!("0x".parse::<ExecutionSeed>().is_err());
        assert!(format!("{HEX}0").parse::<ExecutionSeed>().is_err());
        assert!("1g".parse::<ExecutionSeed>().is_err());
    }

    #[test]
    fn seed_flag_parses_through_from_str() {
        #[derive(clap::Parser)]
        struct Cli {
            #[command(flatten)]
            seed: SeedArgs,
        }
        let cli = <Cli as clap::Parser>::try_parse_from(["cmd", "--seed", "0x10"]).unwrap();
        assert_eq!(cli.seed.seed, Some("10".parse().unwrap()));
        assert!(<Cli as clap::Parser>::try_parse_from(["cmd", "--seed", "1g"]).is_err());
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
