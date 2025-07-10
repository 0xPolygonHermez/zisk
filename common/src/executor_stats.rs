use std::{
    fs,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use zisk_pil::*;

#[derive(Debug, Clone)]
pub enum ExecutorStatsEnum {
    GenerateMT(ExecutorStatsDuration),
    ChunkPlayerMT(ExecutorStatsDuration),
    MTChunkDone(ExecutorStatsDuration),
    End(ExecutorStatsDuration),
    Air(ExecutorStatsAir),
}

#[derive(Debug, Clone)]
pub struct ExecutorStatsDuration {
    pub start_time: Instant,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct ExecutorStatsAir {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub collect: ExecutorStatsDuration,
    pub witness: ExecutorStatsDuration,
    pub num_chunks: usize,
}

#[derive(Debug)]
pub struct ExecutorStats {
    pub start_time: Instant,
    pub stats: Vec<ExecutorStatsEnum>,
}

impl ExecutorStats {
    pub fn new() -> Self {
        Self { start_time: Instant::now(), stats: Vec::new() }
    }

    pub fn add_stat(&mut self, stats: ExecutorStatsEnum) {
        self.stats.push(stats);
    }

    pub fn set_start_time(&mut self, start_time: Instant) {
        self.start_time = start_time;
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
            val if val == SHA_256_F_TABLE_AIR_IDS[0] => "SHA_256_F_TABLE".to_string(),
            val if val == SPECIFIED_RANGES_AIR_IDS[0] => "SPECIFIED_RANGES".to_string(),
            _ => format!("Unknown air_id: {air_id}"),
        }
    }

    /// Stores stats in JSON and CSV file formats
    pub fn store_stats(&self) {
        #[derive(Serialize, Deserialize, Debug)]
        struct Task {
            name: String,
            start: u64,
            duration: u64,
        }
        let mut tasks: Vec<Task> = Vec::new();

        let start_time = self.start_time;
        let stats = &self.stats;

        tracing::info!("Collected a total of {} statistics", stats.len());
        for stat in stats.iter() {
            match stat {
                ExecutorStatsEnum::GenerateMT(stat_duration) => {
                    let name = "MT_GENERATION".to_string();
                    let start =
                        stat_duration.start_time.duration_since(start_time).as_nanos() as u64;
                    let duration = stat_duration.duration.as_nanos() as u64;
                    tasks.push(Task { name, start, duration });
                }
                ExecutorStatsEnum::ChunkPlayerMT(stat_duration) => {
                    let name = "MT_CHUNK_PLAYER".to_string();
                    let start =
                        stat_duration.start_time.duration_since(start_time).as_nanos() as u64;
                    let duration = stat_duration.duration.as_nanos() as u64;
                    tasks.push(Task { name, start, duration });
                }
                ExecutorStatsEnum::MTChunkDone(_stat_duration) => {
                    // let name = "MT_CHUNK_DONE".to_string();
                    // let start =
                    //     stat_duration.start_time.duration_since(start_time).as_nanos() as u64;
                    // let duration = stat_duration.duration.as_nanos() as u64;
                    // tasks.push(Task { name, start, duration });
                }
                ExecutorStatsEnum::End(stat_duration) => {
                    let name = "END".to_string();
                    let start =
                        stat_duration.start_time.duration_since(start_time).as_nanos() as u64;
                    let duration = stat_duration.duration.as_nanos() as u64;
                    tasks.push(Task { name, start, duration });
                }
                ExecutorStatsEnum::Air(stat_air) => {
                    let collect_start_time: u64 =
                        stat_air.collect.start_time.duration_since(start_time).as_nanos() as u64;
                    let collect_duration: u64 = stat_air.collect.duration.as_nanos() as u64;
                    let witness_start_time: u64 =
                        stat_air.witness.start_time.duration_since(start_time).as_nanos() as u64;
                    let witness_duration: u64 = stat_air.witness.duration.as_nanos() as u64;
                    let name = Self::air_name(stat_air.airgroup_id, stat_air.air_id);
                    if collect_duration > 0 {
                        let name = name.clone() + "_collect";
                        let task =
                            Task { name, start: collect_start_time, duration: collect_duration };
                        tasks.push(task);
                    }
                    if witness_duration > 0 {
                        let name = name.clone() + "_witness";
                        let task =
                            Task { name, start: witness_start_time, duration: witness_duration };
                        tasks.push(task);
                    }
                }
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

    /// Stores stats in JSON and CSV file formats
    pub fn print_stats(&self) {
        let start_time = self.start_time;
        let stats = &self.stats;

        println!("Collected a total of {} statistics", stats.len());
        for stat in stats.iter() {
            match stat {
                ExecutorStatsEnum::GenerateMT(stat_duration) => {
                    let name = "MT_GENERATION".to_string();
                    let start =
                        stat_duration.start_time.duration_since(start_time).as_nanos() as u64;
                    let duration = stat_duration.duration.as_nanos() as u64;
                    println!("{} start = {} duration = {}", name, start, duration);
                }
                ExecutorStatsEnum::ChunkPlayerMT(stat_duration) => {
                    let name = "MT_CHUNK_PLAYER".to_string();
                    let start =
                        stat_duration.start_time.duration_since(start_time).as_nanos() as u64;
                    let duration = stat_duration.duration.as_nanos() as u64;
                    println!("{} start = {} duration = {}", name, start, duration);
                }
                ExecutorStatsEnum::MTChunkDone(stat_duration) => {
                    let name = "MT_CHUNK_DONE".to_string();
                    let start =
                        stat_duration.start_time.duration_since(start_time).as_nanos() as u64;
                    let duration = stat_duration.duration.as_nanos() as u64;
                    println!("{} start = {} duration = {}", name, start, duration);
                }
                ExecutorStatsEnum::End(stat_duration) => {
                    let name = "END".to_string();
                    let start =
                        stat_duration.start_time.duration_since(start_time).as_nanos() as u64;
                    let duration = stat_duration.duration.as_nanos() as u64;
                    println!("{} start = {} duration = {}", name, start, duration);
                }
                ExecutorStatsEnum::Air(stat_air) => {
                    let collect_start_time: u64 =
                        stat_air.collect.start_time.duration_since(start_time).as_nanos() as u64;
                    let collect_duration: u64 = stat_air.collect.duration.as_nanos() as u64;
                    let witness_start_time: u64 =
                        stat_air.witness.start_time.duration_since(start_time).as_nanos() as u64;
                    let witness_duration: u64 = stat_air.witness.duration.as_nanos() as u64;
                    let name = Self::air_name(stat_air.airgroup_id, stat_air.air_id);
                    if collect_duration > 0 {
                        let name = name.clone() + "_collect";
                        println!(
                            "{} start = {} duration = {}",
                            name, collect_start_time, collect_duration
                        );
                    }
                    if witness_duration > 0 {
                        let name = name.clone() + "_witness";
                        println!(
                            "{} start = {} duration = {}",
                            name, witness_start_time, witness_duration
                        );
                    }
                }
            }
        }
    }
}
