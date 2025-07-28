use anyhow::Result;
use asm_runner::{AsmRunnerOptions, AsmServices};
use clap::Parser;
use colored::Colorize;
use executor::{Stats, ZiskExecutionResult};
use fields::Goldilocks;
use libloading::{Library, Symbol};
use proofman::ProofMan;
use proofman_common::{json_to_debug_instances_map, DebugInfo, ParamsGPU, ProofOptions};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, fs, path::PathBuf, thread, time::Instant};
use zisk_common::{ExecutorStats, ZiskLibInitFn};
use zisk_pil::*;

use crate::{
    commands::{
        cli_fail_if_macos, get_proving_key, get_witness_computation_lib, initialize_mpi, Field,
    },
    ux::print_banner,
    ZISK_VERSION_MESSAGE,
};

#[cfg(distributed)]
use mpi::traits::*;

#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskStats {
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

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    #[clap(long, default_value_t = Field::Goldilocks)]
    pub field: Field,

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
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'n', long)]
    pub number_threads_witness: Option<usize>,

    #[clap(short = 'x', long)]
    pub max_witness_stored: Option<usize>,

    #[clap(short = 'c', long)]
    pub chunk_size_bits: Option<u64>,

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,

    // PRECOMPILES OPTIONS
    #[clap(long)]
    pub mpi_node: Option<usize>,

    #[clap(long, default_value_t = false)]
    pub minimal_memory: bool,
}

impl ZiskStats {
    pub fn run(&mut self) -> Result<()> {
        cli_fail_if_macos()?;

        print_banner();

        let proving_key = get_proving_key(self.proving_key.as_ref());

        let debug_info = match &self.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                json_to_debug_instances_map(proving_key.clone(), debug_value.clone())
            }
        };

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

        let mut gpu_params = ParamsGPU::new(false);
        gpu_params.with_max_number_streams(1);
        if self.number_threads_witness.is_some() {
            gpu_params.with_number_threads_pools_witness(self.number_threads_witness.unwrap());
        }
        if self.max_witness_stored.is_some() {
            gpu_params.with_max_witness_stored(self.max_witness_stored.unwrap());
        }

        let proofman;
        let mpi_context = initialize_mpi()?;

        proofman_common::initialize_logger(self.verbose.into(), Some(mpi_context.world_rank));

        let world_ranks;

        let world_rank = mpi_context.world_rank;
        let local_rank = mpi_context.local_rank;
        #[cfg(distributed)]
        {
            let world = mpi_context.universe.world();
            world_ranks = world.size() as usize;

            if let Some(mpi_node) = self.mpi_node {
                let m2 = mpi_node as i32 * 2;
                if mpi_context.world_rank < m2 || mpi_context.world_rank >= m2 + 2 {
                    world.split_shared(mpi_context.world_rank);
                    world.barrier();
                    println!(
                        "{}: {}",
                        format!("Rank {}", mpi_context.world_rank).bright_yellow().bold(),
                        "Exiting stats command.".bright_yellow()
                    );
                    return Ok(());
                }
            }

            proofman = ProofMan::<Goldilocks>::new(
                proving_key,
                custom_commits_map,
                true,
                false,
                false,
                gpu_params,
                self.verbose.into(),
                Some(mpi_context.universe),
            )
            .expect("Failed to initialize proofman");
        }

        #[cfg(not(distributed))]
        {
            proofman = ProofMan::<Goldilocks>::new(
                proving_key,
                custom_commits_map,
                true,
                false,
                false,
                gpu_params,
                self.verbose.into(),
            )
            .expect("Failed to initialize proofman");
            world_ranks = 1;
        }

        let mut witness_lib;

        let asm_services = AsmServices::new(world_rank, local_rank, self.port);
        let asm_runner_options = AsmRunnerOptions::new()
            .with_verbose(self.verbose > 0)
            .with_base_port(self.port)
            .with_world_rank(world_rank)
            .with_local_rank(local_rank)
            .with_unlock_mapped_memory(self.unlock_mapped_memory);

        if self.asm.is_some() {
            // Start ASM microservices
            tracing::info!(">>> [{}] Starting ASM microservices.", mpi_context.world_rank,);

            asm_services.start_asm_services(self.asm.as_ref().unwrap(), asm_runner_options)?;
        }

        match self.field {
            Field::Goldilocks => {
                let library = unsafe {
                    Library::new(get_witness_computation_lib(self.witness_lib.as_ref()))?
                };
                let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
                    unsafe { library.get(b"init_library")? };
                witness_lib = witness_lib_constructor(
                    self.verbose.into(),
                    self.elf.clone(),
                    self.asm.clone(),
                    asm_rom,
                    self.chunk_size_bits,
                    Some(world_rank),
                    Some(local_rank),
                    self.port,
                    self.unlock_mapped_memory,
                )
                .expect("Failed to initialize witness library");

                proofman.register_witness(&mut *witness_lib, library);

                proofman
                    .compute_witness_from_lib(
                        self.input.clone(),
                        &debug_info,
                        ProofOptions::new(
                            false,
                            false,
                            false,
                            false,
                            self.minimal_memory,
                            false,
                            PathBuf::new(),
                        ),
                    )
                    .map_err(|e| anyhow::anyhow!("Error generating stats: {}", e))?;
            }
        };

        let (_, stats): (ZiskExecutionResult, Arc<Mutex<ExecutorStats>>) = *witness_lib
            .get_execution_result()
            .ok_or_else(|| anyhow::anyhow!("No execution result found"))?
            .downcast::<(ZiskExecutionResult, Arc<Mutex<ExecutorStats>>)>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast execution result"))?;

        if world_rank % 2 == 1 {
            thread::sleep(std::time::Duration::from_millis(2000));
        }
        tracing::info!("");
        tracing::info!(
            "{} {}",
            format!("--- STATS SUMMARY RANK {}/{}", world_rank, world_ranks),
            "-".repeat(55)
        );

        stats.lock().unwrap().print_stats();

        if self.asm.is_some() {
            // Shut down ASM microservices
            tracing::info!("<<< [{}] Shutting down ASM microservices.", world_rank);
            asm_services.stop_asm_services()?;
        }

        Ok(())
    }

    fn print_command_info(&self) {
        // Print Stats command info
        println!("{} Stats", format!("{: >12}", "Command").bright_green().bold());
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

        let std_mode = if self.debug.is_some() { "Debug mode" } else { "Standard mode" };
        println!("{: >12} {}", "STD".bright_green().bold(), std_mode);
        // println!("{}", format!("{: >12} {}", "Distributed".bright_green().bold(), "ON (nodes: 4, threads: 32)"));

        println!();
    }

    /// Prints stats individually and grouped, with aligned columns.
    ///
    /// # Arguments
    /// * `stats_mutex` - A reference to the Mutex holding the stats vector.
    pub fn print_stats(stats: &[(usize, usize, Stats)]) {
        println!("    Number of airs: {}", stats.len());
        println!();
        println!("    Stats by Air:");
        println!(
            "    {:<8} {:<25} {:<8} {:<12} {:<12}",
            "air id", "Name", "chunks", "collect (ms)", "witness (ms)",
        );
        println!("    {}", "-".repeat(70));

        // Sort individual stats by (airgroup_id, air_id)
        let mut sorted_stats = stats.to_vec();
        sorted_stats.sort_by_key(|(airgroup_id, air_id, _)| (*airgroup_id, *air_id));

        let mut total_collect_time = 0;
        let mut total_witness_time = 0;
        for (airgroup_id, air_id, stats) in sorted_stats.iter() {
            println!(
                "    {:<8} {:<25} {:<8} {:<12} {:<12}",
                air_id,
                Self::air_name(*airgroup_id, *air_id),
                stats.num_chunks,
                stats.collect_duration,
                stats.witness_duration,
            );
            // Accumulate total times
            total_collect_time += stats.collect_duration;
            total_witness_time += stats.witness_duration;
        }

        // Group stats
        let mut grouped: HashMap<(usize, usize), Vec<Stats>> = HashMap::new();
        for (airgroup_id, air_id, stats) in stats.iter() {
            grouped.entry((*airgroup_id, *air_id)).or_default().push(stats.clone());
        }

        println!();
        println!("    Grouped Stats:");
        println!(
            "    {:<8} {:<25}   {:<6}   {:<20}   {:<20}   {:<20}",
            "Air id", "Name", "Count", "Chunks", "Collect (ms)", "Witness (ms)",
        );
        println!(
            "    {:<8} {:<25}   {:<6}   {:<6} {:<6} {:<6}   {:<6} {:<6} {:<6}   {:<6} {:<6} {:<6}",
            "", "", "", "min", "max", "avg", "min", "max", "avg", "min", "max", "avg",
        );
        println!("    {}", "-".repeat(109));

        let mut grouped_sorted: Vec<_> = grouped.into_iter().collect();
        grouped_sorted.sort_by_key(|((airgroup_id, air_id), _)| (*airgroup_id, *air_id));

        for ((airgroup_id, air_id), entries) in grouped_sorted {
            let count = entries.len() as u64;

            let (mut c_min, mut c_max, mut c_sum) = (u64::MAX, 0, 0);
            let (mut w_min, mut w_max, mut w_sum) = (u64::MAX, 0, 0);
            let (mut n_min, mut n_max, mut n_sum) = (usize::MAX, 0, 0usize);

            for e in &entries {
                c_min = c_min.min(e.collect_duration);
                c_max = c_max.max(e.collect_duration);
                c_sum += e.collect_duration;

                w_min = w_min.min(e.witness_duration);
                w_max = w_max.max(e.witness_duration);
                w_sum += e.witness_duration;

                n_min = n_min.min(e.num_chunks);
                n_max = n_max.max(e.num_chunks);
                n_sum += e.num_chunks;
            }

            println!(
                "    {:<8} {:<25} | {:<6} | {:<6} {:<6} {:<6} | {:<6} {:<6} {:<6} | {:<6} {:<6} {:<6}",
                air_id,
                Self::air_name(airgroup_id, air_id),
                count,
                n_min,
                n_max,
                n_sum as u64 / count,
                c_min,
                c_max,
                c_sum / count,
                w_min,
                w_max,
                w_sum / count,
            );
        }
        println!();
        println!("    Total Stats:");
        println!(
            "    Collect: {:10}ms Witness: {:10}ms Total: {:10}ms",
            total_collect_time,
            total_witness_time,
            total_collect_time + total_witness_time
        );
    }

    fn air_name(_airgroup_id: usize, air_id: usize) -> String {
        match air_id {
            val if val == MAIN_AIR_IDS[0] => "Main".to_string(),
            val if val == ROM_AIR_IDS[0] => "ROM".to_string(),
            val if val == MEM_AIR_IDS[0] => "MEM".to_string(),
            val if val == ROM_DATA_AIR_IDS[0] => "ROM_DATA".to_string(),
            val if val == INPUT_DATA_AIR_IDS[0] => "INPUT_DATA".to_string(),
            val if val == MEM_ALIGN_AIR_IDS[0] => "MEM_ALIGN".to_string(),
            val if val == MEM_ALIGN_ROM_AIR_IDS[0] => "MEM_ALIGN_ROM".to_string(),
            val if val == ARITH_AIR_IDS[0] => "ARITH".to_string(),
            val if val == ARITH_TABLE_AIR_IDS[0] => "ARITH_TABLE".to_string(),
            val if val == ARITH_RANGE_TABLE_AIR_IDS[0] => "ARITH_RANGE_TABLE".to_string(),
            val if val == ARITH_EQ_AIR_IDS[0] => "ARITH_EQ".to_string(),
            val if val == ARITH_EQ_LT_TABLE_AIR_IDS[0] => "ARITH_EQ_LT_TABLE".to_string(),
            val if val == BINARY_AIR_IDS[0] => "BINARY".to_string(),
            val if val == BINARY_ADD_AIR_IDS[0] => "BINARY_ADD".to_string(),
            val if val == BINARY_TABLE_AIR_IDS[0] => "BINARY_TABLE".to_string(),
            val if val == BINARY_EXTENSION_AIR_IDS[0] => "BINARY_EXTENSION".to_string(),
            val if val == BINARY_EXTENSION_TABLE_AIR_IDS[0] => "BINARY_EXTENSION_TABLE".to_string(),
            val if val == KECCAKF_AIR_IDS[0] => "KECCAKF".to_string(),
            val if val == KECCAKF_TABLE_AIR_IDS[0] => "KECCAKF_TABLE".to_string(),
            val if val == SHA_256_F_AIR_IDS[0] => "SHA_256_F".to_string(),
            val if val == SPECIFIED_RANGES_AIR_IDS[0] => "SPECIFIED_RANGES".to_string(),
            _ => format!("Unknown air_id: {air_id}"),
        }
    }

    /// Stores stats in JSON file format
    ///
    /// # Arguments
    /// * `stats` - A reference to the stats vector.
    pub fn store_stats(start_time: Instant, stats: &[(usize, usize, Stats)]) {
        #[derive(Serialize, Deserialize, Debug)]
        struct Task {
            name: String,
            start: u64,
            duration: u64,
        }
        let mut tasks: Vec<Task> = Vec::new();

        println!("stats.len={}", stats.len());
        for stat in stats.iter() {
            let airgroup_id = stat.0;
            let air_id = stat.1;
            let stat = &stat.2;
            let collect_start_time: u64 =
                stat.collect_start_time.duration_since(start_time).as_micros() as u64;
            let witness_start_time: u64 =
                stat.witness_start_time.duration_since(start_time).as_micros() as u64;
            let name = ZiskStats::air_name(airgroup_id, air_id);
            if stat.collect_duration > 0 {
                let name = name.clone() + "_collect";
                // println!(
                //     "{} num_chunks={} start_time={}, duration={}",
                //     name, stat.num_chunks, collect_start_time, stat.collect_duration
                // );
                let task =
                    Task { name, start: collect_start_time, duration: stat.collect_duration };
                tasks.push(task);
            }
            if stat.witness_duration > 0 {
                let name = name.clone() + "_witness";
                // println!(
                //     "{} num_chunks={}, start_time={}, duration={}",
                //     name, stat.num_chunks, witness_start_time, stat.witness_duration
                // );
                let task =
                    Task { name, start: witness_start_time, duration: stat.witness_duration };
                tasks.push(task);
            }
        }

        // Save to stats.json

        // Convert to pretty-printed JSON
        let json = serde_json::to_string_pretty(&tasks).unwrap();

        // Write to file
        let _ = fs::write("stats.json", json);

        // Save to stats.csv

        // Create a CSV-formatted string with the tasks data
        let mut csv = String::new();
        for task in tasks {
            csv += &format!("{},{},{},\n", task.name, task.start, task.duration);
        }

        // Write to file
        let _ = fs::write("stats.csv", csv);

        tracing::info!("Statistics have been saved to stats.json and stats.csv");
    }
}
