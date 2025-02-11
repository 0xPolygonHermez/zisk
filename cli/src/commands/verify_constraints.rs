use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use p3_goldilocks::Goldilocks;
use proofman::ProofMan;
use proofman_common::ProofOptions;
use proofman_common::{initialize_logger, json_to_debug_instances_map, DebugInfo};
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser, Debug, Clone, ValueEnum)]
pub enum Field {
    Goldilocks,
    // Add other variants here as needed
}

impl FromStr for Field {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "goldilocks" => Ok(Field::Goldilocks),
            // Add parsing for other variants here
            _ => Err(format!("'{}' is not a valid value for Field", s)),
        }
    }
}

impl Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Field::Goldilocks => write!(f, "goldilocks"),
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskVerifyConstraintsCmd {
    /// Witness computation dynamic library path
    #[clap(short, long)]
    pub witness_lib: PathBuf,

    /// ROM file path
    /// This is the path to the ROM file that the witness computation dynamic library will use
    /// to generate the witness.
    #[clap(short, long)]
    pub rom: Option<PathBuf>,

    /// Inputs path
    #[clap(short = 'i', long)]
    pub input_data: Option<PathBuf>,

    /// Public inputs path
    #[clap(short = 'p', long)]
    pub public_inputs: Option<PathBuf>,

    /// Setup folder path
    #[clap(long)]
    pub proving_key: PathBuf,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

    /// Verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,
}

impl ZiskVerifyConstraintsCmd {
    pub fn run(&self) -> Result<()> {
        println!("{} VerifyConstraints", format!("{: >12}", "Command").bright_green().bold());
        println!();

        initialize_logger(self.verbose.into());

        let debug_info = match &self.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                json_to_debug_instances_map(self.proving_key.clone(), debug_value.clone())
            }
        };

        match self.field {
            Field::Goldilocks => {
                ProofMan::<Goldilocks>::generate_proof(
                    self.witness_lib.clone(),
                    self.rom.clone(),
                    self.public_inputs.clone(),
                    self.input_data.clone(),
                    self.proving_key.clone(),
                    PathBuf::new(),
                    ProofOptions::new(true, self.verbose.into(), false, false, debug_info),
                )
                .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;
            }
        }

        Ok(())
    }
}
