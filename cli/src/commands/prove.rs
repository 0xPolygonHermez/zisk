use crate::{
    commands::{get_proving_key, get_witness_computation_lib, Field},
    ux::print_banner,
    ZISK_VERSION_MESSAGE,
};
use anyhow::Result;
use asm_runner::{AsmRunnerOptions, AsmServices};
use bytemuck::cast_slice;
use colored::Colorize;
use fields::Goldilocks;
use libloading::{Library, Symbol};
use proofman::ProofMan;
use proofman::{ProofInfo, ProvePhase, ProvePhaseInputs, ProvePhaseResult};
use proofman_common::{
    initialize_logger, json_to_debug_instances_map, DebugInfo, ModeName, ParamsGPU, ProofOptions,
};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use std::io::Write;
use std::{
    collections::HashMap,
    fs::{self, File},
    path::PathBuf,
};
#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;
use zisk_common::{ExecutorStats, ProofLog, ZiskExecutionResult, ZiskLibInitFn};
use zstd::stream::write::Encoder;

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

    /// Base port for Assembly microservices (default: 23115).
    /// A single execution will use 3 consecutive ports, from this port to port + 2.
    /// If you are running multiple instances of ZisK using mpi on the same machine,
    /// it will use from this base port to base port + 2 * number_of_instances.
    /// For example, if you run 2 mpi instances of ZisK, it will use ports from 23115 to 23117
    /// for the first instance, and from 23118 to 23120 for the second instance.
    #[clap(short = 'p', long, conflicts_with = "emulator")]
    pub port: Option<u16>,

    /// Map unlocked flag
    /// This is used to unlock the memory map for the ROM file.
    /// If you are running ZisK on a machine with limited memory, you may want to enable this option.
    /// This option is mutually exclusive with `--emulator`.
    #[clap(short = 'u', long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Verbosity (-v, -vv)
    #[arg(short ='v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,

    #[clap(short = 't', long)]
    pub max_streams: Option<usize>,

    #[clap(short = 'n', long)]
    pub number_threads_witness: Option<usize>,

    #[clap(short = 'x', long)]
    pub max_witness_stored: Option<usize>,

    #[clap(short = 'b', long, default_value_t = false)]
    pub save_proofs: bool,

    #[clap(short = 'm', long, default_value_t = false)]
    pub minimal_memory: bool,

    #[clap(short = 'j', long, default_value_t = false)]
    pub shared_tables: bool,
}

impl ZiskProve {
    pub fn run(&mut self) -> Result<()> {
        print_banner();

        let proving_key = get_proving_key(self.proving_key.as_ref());

        let debug_info = match &self.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                json_to_debug_instances_map(proving_key.clone(), debug_value.clone())
            }
        };

        if self.output_dir.join("proofs").exists() {
            // In distributed mode two different processes may enter here at the same time and try to remove the same directory
            if let Err(e) = fs::remove_dir_all(self.output_dir.join("proofs")) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    panic!("Failed to remove the proofs directory: {e:?}");
                }
            }
        }

        if let Err(e) = fs::create_dir_all(self.output_dir.join("proofs")) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                // prevent collision in distributed mode
                panic!("Failed to create the proofs directory: {e:?}");
            }
        }

        let default_cache_path =
            std::env::var("HOME").ok().map(PathBuf::from).unwrap().join(DEFAULT_CACHE_PATH);

        if !default_cache_path.exists() {
            if let Err(e) = fs::create_dir_all(default_cache_path.clone()) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    // prevent collision in distributed mode
                    panic!("Failed to create the cache directory: {e:?}");
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
            let asm_rom_filename = format!("{stem}-{hash}-rh.bin");
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

        if let Some(input) = &self.input {
            if !input.exists() {
                return Err(anyhow::anyhow!("Input file not found at {:?}", input.display()));
            }
        }

        let blowup_factor = get_rom_blowup_factor(&proving_key);

        let rom_bin_path =
            get_elf_bin_file_path(&self.elf.to_path_buf(), &default_cache_path, blowup_factor)?;

        if !rom_bin_path.exists() {
            let _ = gen_elf_hash(&self.elf.clone(), rom_bin_path.as_path(), blowup_factor, false)
                .map_err(|e| anyhow::anyhow!("Error generating elf hash: {}", e));
        }

        self.print_command_info();

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
        if self.max_witness_stored.is_some() {
            gpu_params.with_max_witness_stored(self.max_witness_stored.unwrap());
        }

        let library =
            unsafe { Library::new(get_witness_computation_lib(self.witness_lib.as_ref()))? };
        let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
            unsafe { library.get(b"init_library")? };
        let mut witness_lib = witness_lib_constructor(
            self.verbose.into(),
            self.elf.clone(),
            self.asm.clone(),
            asm_rom,
            self.port,
            self.unlock_mapped_memory,
            self.shared_tables,
        )
        .expect("Failed to initialize witness library");

        let proofman = ProofMan::<Goldilocks>::new(
            proving_key,
            custom_commits_map,
            verify_constraints,
            self.aggregation,
            self.final_snark,
            gpu_params,
            self.verbose.into(),
            witness_lib.get_packed_info(),
        )
        .expect("Failed to initialize proofman");

        let world_rank = proofman.get_world_rank();
        let local_rank = proofman.get_local_rank();

        initialize_logger(self.verbose.into(), Some(world_rank));

        let asm_services = AsmServices::new(world_rank, local_rank, self.port);
        let asm_runner_options = AsmRunnerOptions::new()
            .with_verbose(self.verbose > 0)
            .with_base_port(self.port)
            .with_world_rank(world_rank)
            .with_local_rank(local_rank)
            .with_unlock_mapped_memory(self.unlock_mapped_memory);

        if self.asm.is_some() {
            timer_start_info!(STARTING_ASM_MICROSERVICES);

            asm_services.start_asm_services(self.asm.as_ref().unwrap(), asm_runner_options)?;
            timer_stop_and_log_info!(STARTING_ASM_MICROSERVICES);
        }

        proofman.register_witness(&mut *witness_lib, library);

        let start = std::time::Instant::now();

        let proof_id;
        let vadcop_final_proof: Option<Vec<u64>>;
        if debug_info.std_mode.name == ModeName::Debug {
            match self.field {
                Field::Goldilocks => {
                    return proofman
                        .verify_proof_constraints_from_lib(self.input.clone(), &debug_info, false)
                        .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e));
                }
            };
        } else {
            match self.field {
                Field::Goldilocks => {
                    proofman.set_barrier();
                    let result = proofman
                        .generate_proof_from_lib(
                            ProvePhaseInputs::Full(ProofInfo::new(
                                self.input.clone(),
                                1,
                                vec![0],
                                0,
                            )),
                            ProofOptions::new(
                                false,
                                self.aggregation,
                                self.final_snark,
                                self.verify_proofs,
                                self.minimal_memory,
                                self.save_proofs,
                                self.output_dir.clone(),
                            ),
                            ProvePhase::Full,
                        )
                        .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))?;

                    (proof_id, vadcop_final_proof) =
                        if let ProvePhaseResult::Full(proof_id, vadcop_final_proof) = result {
                            (proof_id, vadcop_final_proof)
                        } else {
                            (None, None)
                        };
                }
            };
        }

        if world_rank == 0 {
            let elapsed = start.elapsed();

            #[allow(clippy::type_complexity)]
            let (result, mut _stats): (ZiskExecutionResult, ExecutorStats) =
                witness_lib.get_execution_result().expect("Failed to get execution result");

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
                let logs = ProofLog::new(result.executed_steps, proof_id, elapsed);
                let log_path = self.output_dir.join("result.json");
                ProofLog::write_json_log(&log_path, &logs)
                    .map_err(|e| anyhow::anyhow!("Error generating log: {}", e))?;

                // Save the uncompressed vadcop final proof
                let output_file_path = self.output_dir.join("vadcop_final_proof.bin");
                let vadcop_proof = vadcop_final_proof.unwrap();
                let mut file = File::create(output_file_path)?;
                file.write_all(cast_slice(&vadcop_proof))?;

                // Save the compressed vadcop final proof using zstd (fastest compression level)
                let compressed_output_path =
                    self.output_dir.join("vadcop_final_proof.compressed.bin");
                let compressed_file = File::create(&compressed_output_path)?;
                let mut encoder = Encoder::new(compressed_file, 1)?;
                encoder.write_all(cast_slice(&vadcop_proof))?;
                encoder.finish()?;

                let original_size = vadcop_proof.len() * 8;
                let compressed_size = std::fs::metadata(&compressed_output_path)?.len();
                let compression_ratio = compressed_size as f64 / original_size as f64;

                println!("Vadcop final proof saved:");
                println!("  Original: {} bytes", original_size);
                println!(
                    "  Compressed: {} bytes (ratio: {:.2}x)",
                    compressed_size, compression_ratio
                );
            }

            // Store the stats in stats.json
            #[cfg(feature = "stats")]
            {
                let stats_id = _stats.next_id();
                _stats.add_stat(0, stats_id, "END", 0, ExecutorStatsEvent::Mark);
                _stats.store_stats();
            }
        }

        proofman.set_barrier();

        if self.asm.is_some() {
            // Shut down ASM microservices
            tracing::info!("<<< [{}] Shutting down ASM microservices.", world_rank);
            asm_services.stop_asm_services()?;
        }

        Ok(())
    }

    fn print_command_info(&self) {
        println!("{} Prove", format!("{: >12}", "Command").bright_green().bold());
        println!(
            "{: >12} {}",
            "Witness Lib".bright_green().bold(),
            get_witness_computation_lib(self.witness_lib.as_ref()).display()
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
            get_proving_key(self.proving_key.as_ref()).display()
        );

        let std_mode = match &self.debug {
            None => "Standard mode",
            Some(None) => "Debug mode (fast)",
            Some(Some(json_file)) => &format!("Debug mode (from config file: {})", json_file),
        };
        println!("{: >12} {}", "STD".bright_green().bold(), std_mode);
        // println!("{}", format!("{: >12} {}", "Distributed".bright_green().bold(), "ON (nodes: 4, threads: 32)"));

        println!();
    }
}
