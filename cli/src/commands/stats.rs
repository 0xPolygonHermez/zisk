use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, time::Instant};
use tracing::warn;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_common::{ExecutorStatsHandle, Stats};
use zisk_pil::*;
use zisk_sdk::ProverClient;

use crate::ux::{print_banner, print_banner_field};

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
    #[clap(short = 'i', long, alias = "input")]
    pub inputs: Option<String>,

    /// Precompiles Hints path
    #[clap(long)]
    pub hints: Option<String>,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

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

    #[clap(short = 'd', long)]
    pub debug: Option<Option<String>>,

    // PRECOMPILES OPTIONS
    #[clap(long)]
    pub mpi_node: Option<usize>,

    #[clap(short = 'm', long, default_value_t = false)]
    pub minimal_memory: bool,

    #[clap(short = 'j', long, default_value_t = false)]
    pub shared_tables: bool,
}

impl ZiskStats {
    pub fn run(&mut self) -> Result<()> {
        // Check if the deprecated alias was used
        if std::env::args().any(|arg| arg == "--input") {
            eprintln!("{}", "Warning: --input is deprecated, use --inputs instead".yellow().bold());
        }

        print_banner();

        if let Some(inputs) = &self.inputs {
            print_banner_field("Input", inputs);
        }

        if let Some(hints) = &self.hints {
            print_banner_field("Prec. Hints", hints);
        }

        let stdin = ZiskStdin::from_uri(self.inputs.as_ref())?;

        let hints_stream = match self.hints.as_ref() {
            Some(uri) => {
                let stream = StreamSource::from_uri(uri)?;
                if matches!(stream, StreamSource::Quic(_)) {
                    anyhow::bail!("QUIC hints source is not supported for execution.");
                }
                Some(stream)
            }
            None => None,
        };

        let emulator = if cfg!(target_os = "macos") {
            if !self.emulator {
                warn!("Emulator mode is forced on macOS due to lack of ASM support.");
            }
            true
        } else {
            self.emulator
        };

        let (world_rank, n_processes, stats) =
            if emulator { self.run_emu(stdin)? } else { self.run_asm(stdin, hints_stream)? };

        if world_rank % 2 == 1 {
            std::thread::sleep(std::time::Duration::from_millis(2000));
        }
        tracing::info!("");
        tracing::info!(
            "{} {}",
            format!("--- STATS SUMMARY RANK {}/{}", world_rank, n_processes),
            "-".repeat(55)
        );

        if let Some(stats) = &stats {
            Self::print_stats(&stats.get_inner().lock().unwrap().witness_stats);
            stats.print_stats();
        }

        Ok(())
    }

    pub fn run_emu(&mut self, stdin: ZiskStdin) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        let prover = ProverClient::builder()
            .emu()
            .witness()
            .proving_key_path_opt(self.proving_key.clone())
            .elf_path(self.elf.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .print_command_info()
            .build()?;

        prover.stats(stdin, None, self.debug.clone(), self.mpi_node.map(|n| n as u32))
    }

    pub fn run_asm(
        &mut self,
        stdin: ZiskStdin,
        hints_stream: Option<StreamSource>,
    ) -> Result<(i32, i32, Option<ExecutorStatsHandle>)> {
        let prover = ProverClient::builder()
            .asm()
            .witness()
            .proving_key_path_opt(self.proving_key.clone())
            .elf_path(self.elf.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .asm_path_opt(self.asm.clone())
            .base_port_opt(self.port)
            .unlock_mapped_memory(self.unlock_mapped_memory)
            .with_hints(hints_stream.is_some())
            .print_command_info()
            .build()?;

        let mpi_node = self.mpi_node.map(|n| n as u32);
        prover.stats(stdin, hints_stream, self.debug.clone(), mpi_node)
    }

    /// Prints stats individually and grouped, with aligned columns.
    ///
    /// # Arguments
    /// * `stats_mutex` - A reference to the Mutex holding the stats vector.
    pub fn print_stats(air_stats: &HashMap<usize, Stats>) {
        println!("    Number of airs: {}", air_stats.len());
        println!();
        println!("    Stats by Air:");
        println!(
            "    {:<8} {:<25} {:<8} {:<12} {:<12}",
            "air id", "Name", "chunks", "collect (ms)", "witness (ms)",
        );
        println!("    {}", "-".repeat(70));

        // Convert HashMap values to flat Vec
        let mut sorted_stats: Vec<&Stats> = air_stats.values().collect();
        sorted_stats.sort_by_key(|stat| (stat.airgroup_id, stat.air_id));

        let mut total_collect_time = 0;
        let mut total_witness_time = 0;
        for stat in sorted_stats.iter() {
            let collect_ms = stat.collect_duration;
            let witness_ms = stat.witness_duration as u64;

            println!(
                "    {:<8} {:<25} {:<8} {:<12} {:<12}",
                stat.air_id,
                Self::air_name(stat.airgroup_id, stat.air_id),
                stat.num_chunks,
                collect_ms,
                witness_ms,
            );
            // Accumulate total times
            total_collect_time += collect_ms;
            total_witness_time += witness_ms;
        }

        // Group stats
        let mut grouped: HashMap<(usize, usize), Vec<&Stats>> = HashMap::new();
        for stat in air_stats.values() {
            grouped.entry((stat.airgroup_id, stat.air_id)).or_default().push(stat);
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
                let collect_ms = e.collect_duration;
                let witness_ms = e.witness_duration as u64;

                c_min = c_min.min(collect_ms);
                c_max = c_max.max(collect_ms);
                c_sum += collect_ms;

                w_min = w_min.min(witness_ms);
                w_max = w_max.max(witness_ms);
                w_sum += witness_ms;

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
            val if val == MEM_ALIGN_BYTE_AIR_IDS[0] => "MEM_ALIGN_BYTE".to_string(),
            val if val == MEM_ALIGN_READ_BYTE_AIR_IDS[0] => "MEM_ALIGN_READ_BYTE".to_string(),
            val if val == MEM_ALIGN_WRITE_BYTE_AIR_IDS[0] => "MEM_ALIGN_WRITE_BYTE".to_string(),
            // val if val == MEM_ALIGN_ROM_AIR_IDS[0] => "MEM_ALIGN_ROM".to_string(),
            val if val == ARITH_AIR_IDS[0] => "ARITH".to_string(),
            // val if val == ARITH_TABLE_AIR_IDS[0] => "ARITH_TABLE".to_string(),
            // val if val == ARITH_RANGE_TABLE_AIR_IDS[0] => "ARITH_RANGE_TABLE".to_string(),
            val if val == ARITH_EQ_AIR_IDS[0] => "ARITH_EQ".to_string(),
            // val if val == ARITH_EQ_LT_TABLE_AIR_IDS[0] => "ARITH_EQ_LT_TABLE".to_string(),
            val if val == BINARY_AIR_IDS[0] => "BINARY".to_string(),
            val if val == BINARY_ADD_AIR_IDS[0] => "BINARY_ADD".to_string(),
            // val if val == BINARY_TABLE_AIR_IDS[0] => "BINARY_TABLE".to_string(),
            val if val == BINARY_EXTENSION_AIR_IDS[0] => "BINARY_EXTENSION".to_string(),
            // val if val == BINARY_EXTENSION_TABLE_AIR_IDS[0] => "BINARY_EXTENSION_TABLE".to_string(),
            val if val == KECCAKF_AIR_IDS[0] => "KECCAKF".to_string(),
            // val if val == KECCAKF_TABLE_AIR_IDS[0] => "KECCAKF_TABLE".to_string(),
            val if val == SHA_256_F_AIR_IDS[0] => "SHA_256_F".to_string(),
            // val if val == SPECIFIED_RANGES_AIR_IDS[0] => "SPECIFIED_RANGES".to_string(),
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
                let task = Task {
                    name,
                    start: witness_start_time,
                    duration: stat.witness_duration as u64,
                };
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
