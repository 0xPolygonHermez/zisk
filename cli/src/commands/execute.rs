use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use fields::Goldilocks;
use libloading::{Library, Symbol};
use proofman::ProofMan;
use proofman_common::ParamsGPU;
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    commands::{cli_fail_if_gpu_mode, cli_fail_if_macos, Field},
    ux::print_banner,
    ZISK_VERSION_MESSAGE,
};
use zisk_common::ZiskLibInitFn;

use super::{get_default_proving_key, get_default_witness_computation_lib};

#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskExecute {
    /// Witness computation dynamic library path
    #[clap(short = 'w', long)]
    pub witness_lib: Option<PathBuf>,

    /// ROM file path
    /// This is the path to the ROM file that the witness computation dynamic library will use
    /// to generate the witness.
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    /// ASM file path
    /// Optional, mutually exclusive with `--emulator`
    #[clap(short = 's', long)]
    pub asm: Option<PathBuf>,

    /// Use prebuilt emulator (mutually exclusive with `--asm`)
    #[clap(short = 'l', long, action = clap::ArgAction::SetTrue)]
    pub emulator: bool,

    /// Input path
    #[clap(short = 'i', long)]
    pub input: Option<PathBuf>,

    #[clap(short = 'o', long)]
    pub output_path: PathBuf,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    // PRECOMPILES OPTIONS
    /// Sha256f script path
    pub sha256f_script: Option<PathBuf>,
}

impl ZiskExecute {
    pub fn run(&mut self) -> Result<()> {
        cli_fail_if_macos()?;
        cli_fail_if_gpu_mode()?;

        let sha256f_script = if let Some(sha256f_path) = &self.sha256f_script {
            sha256f_path.clone()
        } else {
            let home_dir = env::var("HOME").expect("Failed to get HOME environment variable");
            let script_path = PathBuf::from(format!("{}/.zisk/bin/sha256f_script.json", home_dir));
            if !script_path.exists() {
                panic!("Sha256f script file not found at {:?}", script_path);
            }
            script_path
        };

        print_banner();

        let default_cache_path =
            std::env::var("HOME").ok().map(PathBuf::from).unwrap().join(DEFAULT_CACHE_PATH);

        if !default_cache_path.exists() {
            if let Err(e) = fs::create_dir_all(default_cache_path.clone()) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    // prevent collision in distributed mode
                    panic!("Failed to create the cache directory: {:?}", e);
                }
            }
        }

        let emulator = if cfg!(target_os = "macos") { true } else { self.emulator };

        let mut asm_rom = None;
        if emulator {
            self.asm = None;
        } else if self.asm.is_none() {
            let stem = self.elf.file_stem().unwrap().to_str().unwrap();
            let hash = get_elf_data_hash(&self.elf)
                .map_err(|e| anyhow::anyhow!("Error computing ELF hash: {}", e))?;
            let new_filename = format!("{stem}-{hash}-mt.bin");
            let asm_rom_filename = format!("{stem}-{hash}-rom.bin");
            asm_rom = Some(default_cache_path.join(asm_rom_filename));
            self.asm = Some(default_cache_path.join(new_filename));
        }

        if let Some(asm_path) = &self.asm {
            if !asm_path.exists() {
                return Err(anyhow::anyhow!("ASM file not found at {:?}", asm_path.display()));
            }
        }

        if let Some(asm_rom) = &asm_rom {
            if !asm_rom.exists() {
                return Err(anyhow::anyhow!("ASM file not found at {:?}", asm_rom.display()));
            }
        }

        let blowup_factor = get_rom_blowup_factor(&self.get_proving_key());

        let rom_bin_path =
            get_elf_bin_file_path(&self.elf.to_path_buf(), &default_cache_path, blowup_factor)?;

        if !rom_bin_path.exists() {
            let _ = gen_elf_hash(&self.elf.clone(), rom_bin_path.as_path(), blowup_factor, false)
                .map_err(|e| anyhow::anyhow!("Error generating elf hash: {}", e));
        }

        self.print_command_info(&sha256f_script);

        let mut custom_commits_map: HashMap<String, PathBuf> = HashMap::new();
        custom_commits_map.insert("rom".to_string(), rom_bin_path);

        let proofman = ProofMan::<Goldilocks>::new(
            self.get_proving_key(),
            custom_commits_map,
            true,
            false,
            false,
            ParamsGPU::default(),
            self.verbose.into(),
            None,
        )
        .expect("Failed to initialize proofman");

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
                    asm_rom,
                    sha256f_script,
                    proofman.get_rank(),
                    None,
                    None,
                )
                .expect("Failed to initialize witness library");

                proofman.register_witness(&mut *witness_lib, library);

                proofman
                    .execute_from_lib(self.input.clone(), self.output_path.clone())
                    .map_err(|e| anyhow::anyhow!("Error generating execution: {}", e))?;
            }
        };

        Ok(())
    }

    fn print_command_info(&self, sha256f_script: &Path) {
        // Print Verify Contraints command info
        println!("{} Execute", format!("{: >12}", "Command").bright_green().bold());
        println!(
            "{: >12} {}",
            "Witness Lib".bright_green().bold(),
            self.get_witness_computation_lib().display()
        );

        println!("{: >12} {}", "Elf".bright_green().bold(), self.elf.display());

        if self.asm.is_some() {
            let asm_path = self.asm.as_ref().unwrap().display();
            println!("{: >12} {}", "ASM runner".bright_green().bold(), asm_path);
        } else {
            println!(
                "{: >12} {}",
                "Emulator".bright_green().bold(),
                "Running in emulator mode".bright_yellow()
            );
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

        println!("{: >12} {}", "Sha256f".bright_green().bold(), sha256f_script.display());
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
