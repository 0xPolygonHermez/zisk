use anyhow::{anyhow, Ok, Result};
use clap::Parser;
use colored::Colorize;
use proofman_common::initialize_logger;
use std::io::Read;
use std::{fs::File, path::PathBuf};

use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;

use proofman::verify_proof;
use proofman_starks_lib_c::get_zkin_ptr_c;

use crate::ZISK_VERSION_MESSAGE;

#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
pub struct ZiskVerify {
    #[clap(short = 'p', long)]
    pub proof: String,

    #[clap(short = 's', long)]
    pub stark_info: String,

    #[clap(short = 'e', long)]
    pub verifier_bin: String,

    #[clap(short = 'k', long)]
    pub verkey: String,

    #[clap(short = 'u', long)]
    pub public_inputs: Option<PathBuf>,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`
}

impl ZiskVerify {
    const NAME: &'static str = "VStark  ";

    pub fn run(&self) -> Result<()> {
        println!("{} ZiskVerify", format!("{: >12}", "Command").bright_green().bold());
        println!();

        initialize_logger(self.verbose.into());

        let p_proof = get_zkin_ptr_c(&self.proof.clone());

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
            Some(
                verkey_json
                    .into_iter()
                    .map(Goldilocks::from_canonical_u64)
                    .collect::<Vec<Goldilocks>>(),
            )
        } else {
            None
        };

        let valid = verify_proof::<Goldilocks>(
            p_proof,
            self.stark_info.clone(),
            self.verifier_bin.clone(),
            self.verkey.clone(),
            publics,
            None,
            None,
        );

        if !valid {
            println!(
                "{}: ··· {}",
                Self::NAME,
                "\u{2717} Stark proof was not verified".bright_red().bold()
            );
            Err(anyhow!("Stark proof was not verified"))
        } else {
            println!(
                "{}:     {}",
                Self::NAME,
                "\u{2713} Stark proof was verified".bright_green().bold()
            );
            Ok(())
        }
    }
}
