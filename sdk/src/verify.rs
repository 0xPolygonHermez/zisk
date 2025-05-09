use crate::common::{PathBufWithDefault, DEFAULT_HOME_DIR};

use colored::Colorize;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::path::PathBuf;

pub static DEFAULT_STARK_INFO_PATH: Lazy<PathBuf> = Lazy::new(|| {
    DEFAULT_HOME_DIR.join(".zisk/provingKey/zisk/vadcop_final/vadcop_final.starkinfo.json")
});

pub static DEFAULT_VERIFIER_BIN_PATH: Lazy<PathBuf> = Lazy::new(|| {
    DEFAULT_HOME_DIR.join(".zisk/provingKey/zisk/vadcop_final/vadcop_final.verifier.bin")
});

pub static DEFAULT_VERIFICATION_KEY_PATH: Lazy<PathBuf> = Lazy::new(|| {
    DEFAULT_HOME_DIR.join(".zisk/provingKey/zisk/vadcop_final/vadcop_final.verkey.json")
});

/// Verify command configuration options.
#[derive(Clone)]
pub struct VerifyConfig {
    /// STARK info file path
    pub stark_info: PathBufWithDefault,

    /// Verifier binary file path
    pub verifier_bin: PathBufWithDefault,

    /// Verification key file path
    pub verification_key: PathBufWithDefault,

    /// Verbosity level (0 = silent, 1 = verbose, 2 = very verbose, etc.).
    pub verbose: u8,
}

impl Default for VerifyConfig {
    fn default() -> Self {
        Self {
            stark_info: PathBufWithDefault::new(None::<PathBuf>, DEFAULT_STARK_INFO_PATH.clone()),
            verifier_bin: PathBufWithDefault::new(
                None::<PathBuf>,
                DEFAULT_VERIFIER_BIN_PATH.clone(),
            ),
            verification_key: PathBufWithDefault::new(
                None::<PathBuf>,
                DEFAULT_VERIFICATION_KEY_PATH.clone(),
            ),
            verbose: 0,
        }
    }
}

impl VerifyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stark_info(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        self.stark_info.set_path(path);
        self
    }

    pub fn verifier_bin(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        self.verifier_bin.set_path(path);
        self
    }

    pub fn verification_key(mut self, path: Option<impl Into<PathBuf>>) -> Self {
        self.verification_key.set_path(path);
        self
    }

    pub fn verbose(mut self, level: u8) -> Self {
        self.verbose = level;
        self
    }
}

/// VerifyResult holds the result of the proof verification process.
#[derive(Serialize)]
pub struct VerifyResult {
    /// Proof verification was valid or not
    pub valid: bool,
}

impl VerifyResult {
    /// Prints the verification result.
    pub fn print(&self) {
        if self.valid {
            println!("VStark:     {}", "\u{2713} Stark proof was verified".bright_green().bold());
        } else {
            // TODO: What means "..."?
            println!("VStark: ··· {}", "\u{2717} Stark proof was not verified".bright_red().bold());
        }
    }
}

/// VerifyContext holds the context for the proof verification process.
#[derive(Clone, Default)]
pub struct VerifyContext {
    /// Proof file path
    pub proof: PathBuf,

    /// Public inputs file path
    pub public_inputs: Option<PathBuf>,

    /// Verify configuration options
    pub config: VerifyConfig,
}

impl VerifyContext {
    pub fn print(&self) {
        println!("{} ZiskVerify", format!("{: >12}", "Command").bright_green().bold());

        println!("{: >12} {}", "Proof".bright_green().bold(), self.proof.display());

        if let Some(publics) = &self.public_inputs {
            println!("{: >12} {}", "Public Inputs".bright_green().bold(), publics.display());
        }
    }
}
