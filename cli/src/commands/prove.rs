use crate::{
    commands::{cli_fail_if_macos, Field, ZiskLibInitFn},
    proof_log,
    ux::print_banner,
    ZISK_VERSION_MESSAGE,
};
use anyhow::Result;
use colored::Colorize;
use executor::Stats;
use executor::ZiskExecutionResult;
use fields::Goldilocks;
use libloading::{Library, Symbol};
use proofman::ProofMan;
use proofman_common::{json_to_debug_instances_map, DebugInfo, ModeName, ParamsGPU, ProofOptions};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use super::{get_default_proving_key, get_default_witness_computation_lib};

// Structure representing the 'prove' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskProve {
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

    /// Input path
    #[clap(short = 'i', long)]
    pub input: Option<PathBuf>,

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

    #[clap(short = 'r', long, default_value_t = false)]
    pub preallocate: bool,

    /// Verbosity (-v, -vv)
    #[arg(short ='v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,

    #[clap(short = 't', long)]
    pub max_streams: Option<usize>,

    #[clap(short = 'n', long)]
    pub number_threads_witness: Option<usize>,

    #[clap(short = 'm', long)]
    pub max_number_witness_pools: Option<usize>,

    #[clap(short = 'x', long)]
    pub max_witness_stored: Option<usize>,

    #[clap(short = 'b', long, default_value_t = false)]
    pub save_proofs: bool,

    // PRECOMPILES OPTIONS
    /// Sha256f script path
    pub sha256f_script: Option<PathBuf>,
}

impl ZiskProve {
    pub fn run(&mut self) -> Result<()> {
        cli_fail_if_macos()?;

        println!("{} Prove", format!("{: >12}", "Command").bright_green().bold());
        println!();

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

        let verify_constraints = debug_info.std_mode.name == ModeName::Debug;

        let mut gpu_params = ParamsGPU::new(self.preallocate);

        if self.max_streams.is_some() {
            gpu_params.with_max_number_streams(self.max_streams.unwrap());
        }
        if self.number_threads_witness.is_some() {
            gpu_params.with_number_threads_pools_witness(self.number_threads_witness.unwrap());
        }
        if self.max_number_witness_pools.is_some() {
            gpu_params.with_max_number_witness_pools(self.max_number_witness_pools.unwrap());
        }
        if self.max_witness_stored.is_some() {
            gpu_params.with_max_witness_stored(self.max_witness_stored.unwrap());
        }

        let proofman = ProofMan::<Goldilocks>::new(
            self.get_proving_key(),
            custom_commits_map,
            verify_constraints,
            self.aggregation,
            self.final_snark,
            gpu_params,
            self.verbose.into(),
            None,
        )
        .expect("Failed to initialize proofman");

        let library = unsafe { Library::new(self.get_witness_computation_lib())? };
        let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
            unsafe { library.get(b"init_library")? };
        let mut witness_lib = witness_lib_constructor(
            self.verbose.into(),
            self.elf.clone(),
            self.asm.clone(),
            asm_rom,
            sha256f_script,
            proofman.get_rank(),
        )
        .expect("Failed to initialize witness library");

        proofman.register_witness(&mut *witness_lib, library);

        let start = std::time::Instant::now();

        let proof_id;
        if debug_info.std_mode.name == ModeName::Debug {
            match self.field {
                Field::Goldilocks => {
                    return proofman
                        .verify_proof_constraints_from_lib(self.input.clone(), &debug_info)
                        .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e));
                }
            };
        } else {
            match self.field {
                Field::Goldilocks => {
                    proof_id = proofman
                        .generate_proof_from_lib(
                            self.input.clone(),
                            ProofOptions::new(
                                false,
                                self.aggregation,
                                self.final_snark,
                                self.verify_proofs,
                                self.save_proofs,
                                self.output_dir.clone(),
                            ),
                        )
                        .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;
                }
            };
        }

        if proofman.get_rank() == Some(0) || proofman.get_rank().is_none() {
            let elapsed = start.elapsed();

            let (result, _): (ZiskExecutionResult, Vec<(usize, usize, Stats)>) = *witness_lib
                .get_execution_result()
                .ok_or_else(|| anyhow::anyhow!("No execution result found"))?
                .downcast::<(ZiskExecutionResult, Vec<(usize, usize, Stats)>)>()
                .map_err(|_| anyhow::anyhow!("Failed to downcast execution result"))?;

            let elapsed = elapsed.as_secs_f64();
            tracing::info!("");
            tracing::info!(
                "{}",
                "--- PROVE SUMMARY ------------------------".bright_green().bold()
            );
            if let Some(proof_id) = &proof_id {
                tracing::info!("      Proof ID: {}", proof_id);
            }
            tracing::info!("    ► Statistics");
            tracing::info!("      time: {} seconds, steps: {}", elapsed, result.executed_steps);

            if let Some(proof_id) = proof_id {
                let logs = proof_log::ProofLog::new(result.executed_steps, proof_id, elapsed);
                let log_path = self.output_dir.join("result.json");
                proof_log::ProofLog::write_json_log(&log_path, &logs)
                    .map_err(|e| anyhow::anyhow!("Error generating log: {}", e))?;
            }
        }

        Ok(())
    }

    fn print_command_info(&self, sha256f_script: &Path) {
        println!("{} Prove", format!("{: >12}", "Command").bright_green().bold());
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
