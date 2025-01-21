// extern crate env_logger;
use clap::Parser;
use proofman_common::initialize_logger;
use std::{fs::File, path::PathBuf};
use std::io::Read;
use colored::Colorize;

use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;

use proofman::verify_proof;
use proofman_starks_lib_c::get_zkin_ptr_c;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct VerifyStark {
    #[clap(short = 'p', long)]
    pub proof: String,

    #[clap(short = 's', long)]
    pub stark_info: String,

    #[clap(short = 'e', long)]
    pub verifier_bin: String,

    #[clap(short = 'k', long)]
    pub verkey: String,

    #[clap(short = 'u', long)]
    pub publics: Option<PathBuf>,

    #[clap(short = 'f', long)]
    pub proof_values: Option<PathBuf>,

    #[clap(short = 'c', long)]
    pub challenges: Option<PathBuf>,

    /// Verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`
}

impl VerifyStark {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} VerifyStark", format!("{: >12}", "Command").bright_green().bold());
        println!();

        initialize_logger(self.verbose.into());

        let p_proof = get_zkin_ptr_c(&self.proof.clone());

        let publics = if let Some(publics) = &self.publics {
            let mut contents = String::new();
            let mut file = File::open(publics).unwrap();

            let _ =
                file.read_to_string(&mut contents).map_err(|err| format!("Failed to read public inputs file: {}", err));
            let verkey_json: Vec<u64> = serde_json::from_str(&contents).unwrap();
            Some(verkey_json.into_iter().map(Goldilocks::from_canonical_u64).collect::<Vec<Goldilocks>>())
        } else {
            None
        };

        let proof_values = if let Some(proof_values) = &self.proof_values {
            let mut contents = String::new();
            let mut file = File::open(proof_values).unwrap();

            let _ =
                file.read_to_string(&mut contents).map_err(|err| format!("Failed to read public inputs file: {}", err));
            let verkey_json: Vec<Vec<u64>> = serde_json::from_str(&contents).unwrap();
            Some(verkey_json.into_iter().flatten().map(Goldilocks::from_canonical_u64).collect::<Vec<Goldilocks>>())
        } else {
            None
        };

        let valid = verify_proof::<Goldilocks>(
            p_proof,
            self.stark_info.clone(),
            self.verifier_bin.clone(),
            self.verkey.clone(),
            publics,
            proof_values,
            None,
        );

        if !valid {
            log::info!("{}: ··· {}", "VStark  ", "\u{2717} Stark proof was not verified".bright_red().bold());
            Err("Stark proof was not verified".into())
        } else {
            log::info!("{}:     {}", "VStark  ", "\u{2713} Stark proof was verified".bright_green().bold());
            Ok(())
        }
    }
}
