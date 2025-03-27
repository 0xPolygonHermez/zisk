use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use executor::ZiskExecutionResult;
use libloading::{Library, Symbol};
use log::info;
use p3_goldilocks::Goldilocks;
use proofman::ProofMan;
use proofman_common::{initialize_logger, json_to_debug_instances_map, DebugInfo, ProofOptions};
use rom_merkle::{gen_elf_hash, get_elf_bin_file_path, get_rom_blowup_factor, DEFAULT_CACHE_PATH};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    commands::{Field, ZiskLibInitFn},
    ux::print_banner,
    ZISK_VERSION_MESSAGE,
};

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

    #[clap(short = 's', long)]
    pub asm: Option<std::path::PathBuf>,

    /// Input path
    #[clap(short = 'i', long)]
    pub input: Option<PathBuf>,

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

    // PRECOMPILES OPTIONS
    /// Keccak script path
    pub keccak_script: Option<PathBuf>,
}

impl ZiskVerifyConstraints {
    pub fn run(&self) -> Result<()> {
        initialize_logger(self.verbose.into());

        let debug_info = match &self.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                json_to_debug_instances_map(self.get_proving_key(), debug_value.clone())
            }
        };

        let keccak_script = if let Some(keccak_path) = &self.keccak_script {
            keccak_path.clone()
        } else {
            let home_dir = env::var("HOME").expect("Failed to get HOME environment variable");
            let script_path = PathBuf::from(format!("{}/.zisk/bin/keccakf_script.json", home_dir));
            if !script_path.exists() {
                panic!("Keccakf script file not found at {:?}", script_path);
            }
            script_path
        };

        print_banner();

        self.print_command_info(&keccak_script);

        let start = std::time::Instant::now();

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

        let mut witness_lib;
        match self.field {
            Field::Goldilocks => {
                let library = unsafe { Library::new(self.get_witness_computation_lib())? };
                let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
                    unsafe { library.get(b"init_library")? };
                witness_lib = witness_lib_constructor(
                    self.verbose.into(),
                    self.elf.clone(),
                    self.asm.clone(),
                    self.input.clone(),
                    keccak_script,
                )
                .expect("Failed to initialize witness library");

                ProofMan::<Goldilocks>::verify_proof_constraints_from_lib(
                    &mut *witness_lib,
                    self.get_proving_key(),
                    PathBuf::new(),
                    custom_commits_map,
                    ProofOptions::new(true, self.verbose.into(), false, false, false, debug_info),
                )
                .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;
            }
        };

        let elapsed = start.elapsed();

        let result: ZiskExecutionResult = *witness_lib
            .get_execution_result()
            .ok_or_else(|| anyhow::anyhow!("No execution result found"))?
            .downcast::<ZiskExecutionResult>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast execution result"))?;

        println!();
        info!(
            "{}",
            "    Zisk: --- VERIFY CONSTRAINTS SUMMARY ------------------------"
                .bright_green()
                .bold()
        );
        info!("              ► Statistics");
        info!(
            "                time: {} seconds, steps: {}",
            elapsed.as_secs_f32(),
            result.executed_steps
        );

        Ok(())
    }

    fn print_command_info(&self, keccak_script: &Path) {
        // Print Verify Contraints command info
        println!("{} VerifyConstraints", format!("{: >12}", "Command").bright_green().bold());
        println!(
            "{: >12} {}",
            "Witness Lib".bright_green().bold(),
            self.get_witness_computation_lib().display()
        );
        println!("{: >12} {}", "Elf".bright_green().bold(), self.elf.display());
        if self.asm.is_some() {
            let asm_path = self.asm.as_ref().unwrap().display();
            println!("{: >12} {}", "ASM runner".bright_green().bold(), asm_path);
        }
        if self.input.is_some() {
            let inputs_path = self.input.as_ref().unwrap().display();
            println!("{: >12} {}", "Inputs".bright_green().bold(), inputs_path);
        }
        println!(
            "{: >12} {}",
            "Proving key".bright_green().bold(),
            self.get_proving_key().display()
        );
        let std_mode = if self.debug.is_some() { "Debug mode" } else { "Standard mode" };
        println!("{: >12} {}", "STD".bright_green().bold(), std_mode);
        println!("{: >12} {}", "Keccak".bright_green().bold(), keccak_script.display());
        // println!("{}", format!("{: >12} {}", "Distributed".bright_green().bold(), "ON (nodes: 4, threads: 32)"));

        println!();
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
