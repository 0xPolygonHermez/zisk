use anyhow::{anyhow, Ok, Result};
use clap::Parser;
use colored::Colorize;
use proofman_common::initialize_logger;
use std::io::Read;
use std::{fs::File, path::PathBuf};

use fields::{Goldilocks, PrimeField64};

use proofman::{verify_proof, verify_proof_from_file};
use bytemuck::cast_slice;

use crate::commands::cli_fail_if_macos;
use crate::ZISK_VERSION_MESSAGE;

use super::{get_default_stark_info, get_default_verifier_bin, get_default_verkey};

#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
pub struct ZiskVerify {
    #[clap(short = 'p', long)]
    pub proof: String,

    #[clap(short = 's', long)]
    pub stark_info: Option<String>,

    #[clap(short = 'e', long)]
    pub verifier_bin: Option<String>,

    #[clap(short = 'k', long)]
    pub verkey: Option<String>,

    #[clap(short = 'u', long)]
    pub public_inputs: Option<PathBuf>,

    #[clap(short = 'j', long, default_value_t = false)]
    pub json: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`
}

impl ZiskVerify {
    pub fn run(&self) -> Result<()> {
        cli_fail_if_macos()?;

        initialize_logger(self.verbose.into(), None);

        tracing::info!(
            "{}",
            format!("{} ZiskVerify", format!("{: >12}", "Command").bright_green().bold())
        );
        tracing::info!("");

        let publics = if let Some(publics) = &self.public_inputs {
            let mut contents = String::new();
            let mut file = File::open(publics).unwrap();

            let _ = file
                .read_to_string(&mut contents)
                .map_err(|err| format!("Failed to read public inputs file: {}", err));
            let verkey_json_string: Vec<String> = serde_json::from_str(&contents).unwrap();
            let verkey_json: Vec<u64> = verkey_json_string
                .iter()
                .map(|s| s.parse::<u64>().expect("Failed to parse string as u64"))
                .collect();
            Some(verkey_json.into_iter().map(Goldilocks::from_u64).collect::<Vec<Goldilocks>>())
        } else {
            None
        };

        let valid = if self.json {
            verify_proof_from_file::<Goldilocks>(
                self.proof.clone(),
                self.get_stark_info(),
                self.get_verifier_bin(),
                self.get_verkey(),
                publics,
                None,
                None,
            )
        } else {
            let mut file = File::open(self.proof.clone())?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;

            let proof_slice: &[u64] = cast_slice(&buffer);
            let proof_ptr = proof_slice.as_ptr() as *mut u64;

            verify_proof::<Goldilocks>(
                proof_ptr,
                self.get_stark_info(),
                self.get_verifier_bin(),
                self.get_verkey(),
                publics,
                None,
                None,
            )
        };

        if !valid {
            tracing::info!(
                "VStark  : ··· {}",
                "\u{2717} Stark proof was not verified".bright_red().bold()
            );
            Err(anyhow!("Stark proof was not verified"))
        } else {
            tracing::info!(
                "VStark  :     {}",
                "\u{2713} Stark proof was verified".bright_green().bold()
            );
            Ok(())
        }
    }

    /// Gets the stark info JSON file location.
    /// Uses the default one if not specified by user.
    pub fn get_stark_info(&self) -> String {
        if self.stark_info.is_none() {
            get_default_stark_info()
        } else {
            self.stark_info.clone().unwrap()
        }
    }

    /// Gets the verifier binary file location.
    /// Uses the default one if not specified by user.
    pub fn get_verifier_bin(&self) -> String {
        if self.verifier_bin.is_none() {
            get_default_verifier_bin()
        } else {
            self.verifier_bin.clone().unwrap()
        }
    }

    /// Gets the verification key JSON file location.
    /// Uses the default one if not specified by user.
    pub fn get_verkey(&self) -> String {
        if self.verkey.is_none() {
            get_default_verkey()
        } else {
            self.verkey.clone().unwrap()
        }
    }
}
