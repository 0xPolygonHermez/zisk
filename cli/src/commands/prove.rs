use std::{fs, path::PathBuf};

use crate::{commands::Field, ZISK_VERSION_MESSAGE};
use anyhow::Result;
use colored::Colorize;
use p3_goldilocks::Goldilocks;
use proofman::ProofMan;
use proofman_common::{json_to_debug_instances_map, DebugInfo, ProofOptions};

// Structure representing the 'prove' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
pub struct ZiskProve {
    /// Witness computation dynamic library path
    #[clap(short, long)]
    pub witness_lib: PathBuf,
    /// ELF file path
    /// This is the path to the ELF file that the witness computation dynamic library will use
    /// to generate the witness.
    #[clap(short = 'e', long)]
    pub elf: Option<PathBuf>,
    /// Inputs path
    #[clap(short = 'i', long)]
    pub inputs: Option<PathBuf>,
    /// Public inputs path
    #[clap(short = 'p', long)]
    pub public_inputs: Option<PathBuf>,
    /// Setup folder path
    #[clap(long)]
    pub proving_key: PathBuf,
    /// Output dir path
    #[clap(short = 'o', long, default_value = "tmp")]
    pub output_dir: PathBuf,
    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,
    #[clap(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,
    #[clap(short = 'f', long, default_value_t = false)]
    pub final_snark: bool,
    /// Verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`
    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,
}

impl ZiskProve {
    pub fn run(&self) -> Result<()> {
        println!("{} Prove", format!("{: >12}", "Command").bright_green().bold());
        println!();

        if self.output_dir.join("proofs").exists() {
            // In distributed mode two different processes may enter here at the same time and try to remove the same directory
            if let Err(e) = fs::remove_dir_all(&self.output_dir.join("proofs")) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    panic!("Failed to remove the proofs directory: {:?}", e);
                }
            }
        }

        if let Err(e) = fs::create_dir_all(&self.output_dir.join("proofs")) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                // prevent collision in distributed mode
                panic!("Failed to create the proofs directory: {:?}", e);
            }
        }

        let debug_info = match &self.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                let proving_key: PathBuf = PathBuf::from(&self.proving_key);
                json_to_debug_instances_map(proving_key, debug_value.clone())
            }
        };

        match self.field {
            Field::Goldilocks => {
                ProofMan::<Goldilocks>::generate_proof(
                    self.witness_lib.clone(),
                    self.elf.clone(),
                    self.public_inputs.clone(),
                    self.inputs.clone(),
                    self.proving_key.clone(),
                    self.output_dir.clone(),
                    ProofOptions::new(
                        false,
                        self.verbose.into(),
                        self.aggregation,
                        self.final_snark,
                        debug_info,
                    ),
                )
                .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;
            }
        }

        Ok(())
    }
}
