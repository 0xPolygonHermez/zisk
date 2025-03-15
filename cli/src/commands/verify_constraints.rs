use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use p3_goldilocks::Goldilocks;
use proofman::ProofMan;
use proofman_common::{initialize_logger, json_to_debug_instances_map, DebugInfo, ProofOptions};
use rom_merkle::{gen_elf_hash, get_elf_bin_file_path, get_rom_blowup_factor, DEFAULT_CACHE_PATH};
use std::{collections::HashMap, fs, path::PathBuf};

use crate::{commands::Field, ZISK_VERSION_MESSAGE};

use super::{get_default_proving_key, get_default_witness_computation_lib};

#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
pub struct ZiskVerifyConstraints {
    /// Witness computation dynamic library path
    #[clap(short = 'w', long)]
    pub witness_lib: Option<PathBuf>,

    /// ROM file path
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

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,
}

impl ZiskVerifyConstraints {
    pub fn run(&self) -> Result<()> {
        println!("{} VerifyConstraints", format!("{: >12}", "Command").bright_green().bold());
        println!();

        initialize_logger(self.verbose.into());

        let debug_info = match &self.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                json_to_debug_instances_map(self.get_proving_key(), debug_value.clone())
            }
        };

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

        match self.field {
            Field::Goldilocks => ProofMan::<Goldilocks>::verify_proof_constraints(
                self.get_witness_computation_lib(),
                Some(self.elf.clone()),
                self.asm.clone(),
                self.public_inputs.clone(),
                self.input.clone(),
                self.get_proving_key(),
                PathBuf::new(),
                custom_commits_map,
                ProofOptions::new(true, self.verbose.into(), false, false, false, debug_info),
            )
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?,
        };

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
