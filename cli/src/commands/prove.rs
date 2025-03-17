use crate::{commands::Field, ux::print_banner, ZISK_VERSION_MESSAGE};
use anyhow::Result;
use colored::Colorize;
use p3_goldilocks::Goldilocks;
use proofman::ProofMan;
use proofman_common::{
    initialize_logger, json_to_debug_instances_map, DebugInfo, ModeName, ProofOptions,
};
use rom_merkle::{gen_elf_hash, get_elf_bin_file_path, get_rom_blowup_factor, DEFAULT_CACHE_PATH};
use std::{collections::HashMap, fs, path::PathBuf};

use super::{get_default_proving_key, get_default_witness_computation_lib};

// Structure representing the 'prove' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
pub struct ZiskProve {
    /// Witness computation dynamic library path
    #[clap(short = 'w', long)]
    pub witness_lib: Option<PathBuf>,

    /// ELF file path
    /// This is the path to the ROM file that the witness computation dynamic library will use
    /// to generate the witness.
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    #[clap(short = 'a', long)]
    pub asm: Option<std::path::PathBuf>,

    /// Input path
    #[clap(short = 'i', long)]
    pub input: Option<PathBuf>,

    /// Public inputs path
    #[clap(short = 'u', long)]
    pub public_inputs: Option<PathBuf>,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Output dir path
    #[clap(short = 'o', long, default_value = "tmp")]
    pub output_dir: PathBuf,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

    #[clap(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    #[clap(short = 'f', long, default_value_t = false)]
    pub final_snark: bool,

    #[clap(short = 'y', long, default_value_t = false)]
    pub verify_proofs: bool,

    /// Verbosity (-v, -vv)
    #[arg(short ='v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,

    #[clap(short = 'c', long)]
    pub default_cache: Option<PathBuf>,
}

impl ZiskProve {
    pub fn run(&self) -> Result<()> {
        println!("{} Prove", format!("{: >12}", "Command").bright_green().bold());
        println!();

        initialize_logger(self.verbose.into());

        let debug_info = match &self.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                let proving_key: PathBuf = PathBuf::from(&self.get_proving_key());
                json_to_debug_instances_map(proving_key, debug_value.clone())
            }
        };

        print_banner();

        println!("{} Prove", format!("{: >12}", "Command").bright_green().bold());
        let witness_lib = self.witness_lib.as_ref().unwrap().display();
        println!("{}", format!("{: >12} {}", "Witness Lib".bright_green().bold(), witness_lib));
        println!("{}", format!("{: >12} {}", "Elf".bright_green().bold(), self.elf.display()));
        // println!("{}", format!("{: >12} {}", "ASM runner".bright_green().bold(), self.asm_runner.as_ref().unwrap_or_else("None").display()));
        let inputs_path = self.input.as_ref().unwrap().display();
        println!("{}", format!("{: >12} {}", "Inputs".bright_green().bold(), inputs_path));
        let proving_key = self.proving_key.as_ref().unwrap().display();
        println!("{}", format!("{: >12} {}", "Proving key".bright_green().bold(), proving_key));
        let std_mode = if self.debug.is_some() { "Debug mode" } else { "Standard mode" };
        println!("{}", format!("{: >12} {}", "STD".bright_green().bold(), std_mode));
        // println!("{}", format!("{: >12} {}", "Distributed".bright_green().bold(), "ON (nodes: 4, threads: 32)"));

        println!();
        
        if self.output_dir.join("proofs").exists() {
            // In distributed mode two different processes may enter here at the same time and try to remove the same directory
            if let Err(e) = fs::remove_dir_all(self.output_dir.join("proofs")) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    panic!("Failed to remove the proofs directory: {:?}", e);
                }
            }
        }

        if let Err(e) = fs::create_dir_all(self.output_dir.join("proofs")) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                // prevent collision in distributed mode
                panic!("Failed to create the proofs directory: {:?}", e);
            }
        }

        let default_cache_path =
            std::env::var("HOME").ok().map(PathBuf::from).unwrap().join(DEFAULT_CACHE_PATH);

        if !default_cache_path.exists() {
            if let Err(e) = fs::create_dir_all(default_cache_path.clone()) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    // prevent collision in distributed mode
                    panic!("Failed to create the proofs directory: {:?}", e);
                }
            }
        }

        let blowup_factor = get_rom_blowup_factor(&self.get_proving_key());

        let rom_bin_path =
            get_elf_bin_file_path(&self.elf.to_path_buf(), &default_cache_path, blowup_factor)?;

        if !rom_bin_path.exists() {
            let _ = gen_elf_hash(
                &self.elf.clone(),
                rom_bin_path.clone().to_str().unwrap(),
                blowup_factor,
                false,
            )
            .map_err(|e| anyhow::anyhow!("Error generating elf hash: {}", e));
        }

        let mut custom_commits_map: HashMap<String, PathBuf> = HashMap::new();
        custom_commits_map.insert("rom".to_string(), rom_bin_path);

        if debug_info.std_mode.name == ModeName::Debug {
            match self.field {
                Field::Goldilocks => ProofMan::<Goldilocks>::verify_proof_constraints(
                    self.get_witness_computation_lib(),
                    Some(self.elf.clone()),
                    self.asm.clone(),
                    self.public_inputs.clone(),
                    self.input.clone(),
                    self.get_proving_key(),
                    self.output_dir.clone(),
                    custom_commits_map,
                    ProofOptions::new(
                        false,
                        self.verbose.into(),
                        self.aggregation,
                        self.final_snark,
                        self.verify_proofs,
                        debug_info,
                    ),
                )
                .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?,
            };
        } else {
            match self.field {
                Field::Goldilocks => ProofMan::<Goldilocks>::generate_proof(
                    self.get_witness_computation_lib(),
                    Some(self.elf.clone()),
                    self.asm.clone(),
                    self.public_inputs.clone(),
                    self.input.clone(),
                    self.get_proving_key(),
                    self.output_dir.clone(),
                    custom_commits_map,
                    ProofOptions::new(
                        false,
                        self.verbose.into(),
                        self.aggregation,
                        self.final_snark,
                        self.verify_proofs,
                        debug_info,
                    ),
                )
                .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?,
            };
        }

        Ok(())
    }

    /// Gets the witness computation library file location.
    /// Uses the default one if not specified by user.
    pub fn get_witness_computation_lib(&self) -> PathBuf {
        if self.witness_lib.is_none() {
            get_default_witness_computation_lib()
        } else {
            self.witness_lib.clone().unwrap()
        }
    }

    /// Gets the proving key file location.
    /// Uses the default one if not specified by user.
    pub fn get_proving_key(&self) -> PathBuf {
        if self.proving_key.is_none() {
            get_default_proving_key()
        } else {
            self.proving_key.clone().unwrap()
        }
    }
}
