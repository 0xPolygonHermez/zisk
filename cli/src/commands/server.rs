use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use proofman_common::{json_to_debug_instances_map, DebugInfo};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use server::{Server, ServerConfig};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::Path;
use std::{env, fs};
use std::{path::PathBuf, process};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::commands::Field;
use crate::ux::print_banner;
use crate::ZISK_VERSION_MESSAGE;

use super::{get_default_proving_key, get_default_witness_computation_lib};

const DEFAULT_PORT: u16 = 7878;
const LOG_PATH: &str = "zisk_prover_server.log";

// Structure representing the 'prove' subcommand of cargo.
#[derive(Parser, Debug)]
#[command(name = "Prover Server", version, about = "A TCP-based prover control server", long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskServer {
    /// Optional port number (default 7878)
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Witness computation dynamic library path
    #[clap(short = 'w', long)]
    pub witness_lib: Option<PathBuf>,

    /// ELF file path
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

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

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

    // PRECOMPILES OPTIONS
    /// Sha256f script path
    pub sha256f_script: Option<PathBuf>,
}

impl ZiskServer {
    pub fn run(&mut self) -> Result<()> {
        init_tracing(LOG_PATH);
        // initialize_logger(self.verbose.into());

        if !self.elf.exists() {
            eprintln!("Error: ELF file '{}' not found.", self.elf.display());
            process::exit(1);
        }

        let debug_info = match &self.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                let proving_key: PathBuf = PathBuf::from(&self.get_proving_key());
                json_to_debug_instances_map(proving_key, debug_value.clone())
            }
        };

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

        let config = ServerConfig::new(
            self.port,
            self.elf.clone(),
            self.get_witness_computation_lib(),
            self.asm.clone(),
            emulator,
            self.get_proving_key(),
            self.aggregation,
            self.final_snark,
            self.verify_proofs,
            self.verbose,
            debug_info,
            sha256f_script,
        );

        if let Err(e) = Server::new(config).run() {
            eprintln!("Error starting server: {}", e);
            process::exit(1);
        }

        Ok(())
    }

    fn print_command_info(&self, sha256f_script: &Path) {
        println!("{} Prove Server", format!("{: >12}", "Command").bright_green().bold());
        println!(
            "{} TCP server listening on 127.0.0.1:{}",
            format!("{: >12}", "Socket").bright_green().bold(),
            self.port
        );
        println!("{} {}", format!("{: >12}", "Logfile").bright_green().bold(), LOG_PATH);
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

        println!(
            "{: >12} {}",
            "Proving key".bright_green().bold(),
            self.get_proving_key().display()
        );

        let std_mode = if self.debug.is_some() { "Debug mode" } else { "Standard mode" };
        println!("{: >12} {}", "STD".bright_green().bold(), std_mode);
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

fn init_tracing(log_path: &str) {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_path)
        .expect("Failed to open log file");

    let file_layer = fmt::layer()
        .with_writer(file)
        .with_ansi(false) // no color in file
        .with_target(false);

    let stdout_layer = fmt::layer().with_writer(std::io::stdout).with_ansi(true).with_target(false);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with(stdout_layer)
        .with(file_layer)
        .init();
}
