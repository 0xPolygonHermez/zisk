use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use executor::{Stats, ZiskExecutionResult};
use libloading::{Library, Symbol};
use p3_goldilocks::Goldilocks;
use proofman::ProofMan;
use proofman_common::{initialize_logger, json_to_debug_instances_map, DebugInfo, ProofOptions};
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
    commands::{cli_fail_if_gpu_mode, cli_fail_if_macos, Field, ZiskLibInitFn},
    ux::print_banner,
    ZISK_VERSION_MESSAGE,
};

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
pub struct ZiskVerifyConstraints {
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

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,

    // PRECOMPILES OPTIONS
    /// Sha256f script path
    pub sha256f_script: Option<PathBuf>,
}

impl ZiskVerifyConstraints {
    pub fn run(&mut self) -> Result<()> {
        cli_fail_if_macos()?;
        cli_fail_if_gpu_mode()?;

        initialize_logger(self.verbose.into());

        let debug_info = match &self.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                json_to_debug_instances_map(self.get_proving_key(), debug_value.clone())
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

        let start = std::time::Instant::now();

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
                    self.input.clone(),
                    sha256f_script,
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

        let (result, stats): (ZiskExecutionResult, Vec<(usize, usize, Stats)>) = *witness_lib
            .get_execution_result()
            .ok_or_else(|| anyhow::anyhow!("No execution result found"))?
            .downcast::<(ZiskExecutionResult, Vec<(usize, usize, Stats)>)>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast execution result"))?;

        println!();
        tracing::info!(
            "{}",
            "--- VERIFY CONSTRAINTS SUMMARY ------------------------".bright_green().bold()
        );
        tracing::info!("    ► Statistics");
        tracing::info!(
            "      time: {} seconds, steps: {}",
            elapsed.as_secs_f32(),
            result.executed_steps
        );

        Self::print_stats(stats);

        Ok(())
    }

    fn print_command_info(&self, sha256f_script: &Path) {
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

    /// Prints stats individually and grouped, with aligned columns.
    ///
    /// # Arguments
    /// * `stats_mutex` - A reference to the Mutex holding the stats vector.
    pub fn print_stats(stats: Vec<(usize, usize, Stats)>) {
        println!("Individual Entries:");
        println!(
            "{:<25} {:<8} {:<15} {:<15} {:<12}",
            "Name", "air id", "collect (ms)", "witness (ms)", "chunks"
        );
        println!("{}", "-".repeat(80));

        // Sort individual stats by (airgroup_id, air_id)
        let mut sorted_stats = stats.clone();
        sorted_stats.sort_by_key(|(airgroup_id, air_id, _)| (*airgroup_id, *air_id));

        for (airgroup_id, air_id, stats) in sorted_stats.iter() {
            println!(
                "{:<25} {:<8} {:<15} {:<15} {:<12}",
                Self::air_name(*airgroup_id, *air_id),
                air_id,
                stats.collect_time,
                stats.witness_time,
                stats.num_chunks,
            );
        }

        // Group stats
        let mut grouped: HashMap<(usize, usize), Vec<Stats>> = HashMap::new();
        for (airgroup_id, air_id, stats) in stats.iter() {
            grouped.entry((*airgroup_id, *air_id)).or_default().push(stats.clone());
        }

        println!("\nGrouped Totals:");
        println!(
            "{:<8} {:<25}   {:<6}   {:<20}   {:<20}   {:<20}",
            "Air id", "Name", "Count", "Chunks", "Collect (ms)", "Witness (ms)",
        );
        println!(
            "{:<8} {:<25}   {:<6}   {:<6} {:<6} {:<6}   {:<6} {:<6} {:<6}   {:<6} {:<6} {:<6}",
            "", "", "", "min", "max", "avg", "min", "max", "avg", "min", "max", "avg",
        );
        println!("{}", "-".repeat(109));

        let mut grouped_sorted: Vec<_> = grouped.into_iter().collect();
        grouped_sorted.sort_by_key(|((airgroup_id, air_id), _)| (*airgroup_id, *air_id));

        for ((airgroup_id, air_id), entries) in grouped_sorted {
            let count = entries.len() as u64;

            let (mut c_min, mut c_max, mut c_sum) = (u64::MAX, 0, 0);
            let (mut w_min, mut w_max, mut w_sum) = (u64::MAX, 0, 0);
            let (mut n_min, mut n_max, mut n_sum) = (usize::MAX, 0, 0usize);

            for e in &entries {
                c_min = c_min.min(e.collect_time);
                c_max = c_max.max(e.collect_time);
                c_sum += e.collect_time;

                w_min = w_min.min(e.witness_time);
                w_max = w_max.max(e.witness_time);
                w_sum += e.witness_time;

                n_min = n_min.min(e.num_chunks);
                n_max = n_max.max(e.num_chunks);
                n_sum += e.num_chunks;
            }

            println!(
                "{:<8} {:<25} | {:<6} | {:<6} {:<6} {:<6} | {:<6} {:<6} {:<6} | {:<6} {:<6} {:<6}",
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
    }

    fn air_name(_airgroup_id: usize, air_id: usize) -> String {
        match air_id {
            0 => "Main".to_string(),
            1 => "ROM".to_string(),
            2 => "MEM".to_string(),
            3 => "ROM_DATA".to_string(),
            4 => "INPUT_DATA".to_string(),
            5 => "MEM_ALIGN".to_string(),
            6 => "MEM_ALIGN_ROM".to_string(),
            7 => "ARITH".to_string(),
            8 => "ARITH_TABLE".to_string(),
            9 => "ARITH_RANGE_TABLE".to_string(),
            10 => "ARITH_EQ".to_string(),
            11 => "ARITH_EQ_LT_TABLE".to_string(),
            12 => "BINARY".to_string(),
            13 => "BINARY_ADD".to_string(),
            14 => "BINARY_TABLE".to_string(),
            15 => "BINARY_EXTENSION".to_string(),
            16 => "BINARY_EXTENSION_TABLE".to_string(),
            17 => "KECCAKF".to_string(),
            18 => "KECCAKF_TABLE".to_string(),
            19 => "SHA_256_F".to_string(),
            20 => "SHA_256_F_TABLE".to_string(),
            21 => "SPECIFIED_RANGES".to_string(),
            _ => format!("Unknown air_id: {}", air_id),
        }
    }
}
