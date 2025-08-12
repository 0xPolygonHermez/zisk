use std::{
    fs, process,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use zisk_pil::*;

#[derive(Debug, Clone)]
pub enum ExecutorStatsEnum {
    ChunkPlayerMT(ExecutorStatsDuration),
    MTChunkDone(ExecutorStatsDuration),
    AsmMtGeneration(ExecutorStatsDuration),
    AsmRomHistogram(ExecutorStatsDuration),
    AsmMemOps(ExecutorStatsDuration),
    AsmWriteInput(ExecutorStatsDuration),
    MemOpsChunkDone(ExecutorStatsDuration),
    MemOpsProcessChunks(ExecutorStatsDuration),
    MemOpsCollectPlans(ExecutorStatsDuration),
    MemOpsCountPhase(ExecutorStatsDuration),
    MemOpsPlanPhase(ExecutorStatsDuration),
    MemOpsExecuteChunk0(ExecutorStatsDuration),
    MemOpsExecuteChunk1(ExecutorStatsDuration),
    MemOpsExecuteChunk2(ExecutorStatsDuration),
    MemOpsExecuteChunk3(ExecutorStatsDuration),
    MemOpsExecuteChunk4(ExecutorStatsDuration),
    MemOpsExecuteChunk5(ExecutorStatsDuration),
    MemOpsExecuteChunk6(ExecutorStatsDuration),
    MemOpsExecuteChunk7(ExecutorStatsDuration),
    PlanGenerationMain(ExecutorStatsDuration),
    PlanGenerationSecondary(ExecutorStatsDuration),
    PlanGenerationMemOpWait(ExecutorStatsDuration),
    PlanGenerationMemOp(ExecutorStatsDuration),
    ConfigureInstances(ExecutorStatsDuration),
    PreCalculateWitness(ExecutorStatsDuration),
    CalculateWitness(ExecutorStatsDuration),
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

impl Default for ExecutorStats {
    fn default() -> Self {
        Self::new()
    }
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
            val if val == SPECIFIED_RANGES_AIR_IDS[0] => "SPECIFIED_RANGES".to_string(),
            _ => format!("Unknown air_id: {air_id}"),
        }
    }

    fn stats_name(stats: &ExecutorStatsEnum) -> String {
        match stats {
            ExecutorStatsEnum::ChunkPlayerMT(_stat_duration) => "MT_CHUNK_PLAYER".to_string(),
            ExecutorStatsEnum::MTChunkDone(_stat_duration) => "MT_CHUNK_DONE".to_string(),
            ExecutorStatsEnum::AsmMtGeneration(_stat_duration) => "ASM_MT_GENERATION".to_string(),
            ExecutorStatsEnum::AsmRomHistogram(_stat_duration) => "ASM_ROM_HISTOGRAM".to_string(),
            ExecutorStatsEnum::AsmMemOps(_stat_duration) => "ASM_MEM_OPS".to_string(),
            ExecutorStatsEnum::AsmWriteInput(_stat_duration) => "ASM_WRITE_INPUT".to_string(),
            ExecutorStatsEnum::MemOpsChunkDone(_stat_duration) => "MEM_OPS_CHUNK_DONE".to_string(),
            ExecutorStatsEnum::MemOpsProcessChunks(_stat_duration) => {
                "MEM_OPS_PROCESS_CHUNKS".to_string()
            }
            ExecutorStatsEnum::MemOpsCollectPlans(_stat_duration) => {
                "MEM_OPS_COLLECT_PLANS".to_string()
            }
            ExecutorStatsEnum::MemOpsCountPhase(_stat_duration) => {
                "MEM_OPS_COUNT_PHASE".to_string()
            }
            ExecutorStatsEnum::MemOpsPlanPhase(_stat_duration) => "MEM_OPS_PLAN_PHASE".to_string(),
            ExecutorStatsEnum::MemOpsExecuteChunk0(_stat_duration) => {
                "MEM_OPS_EXECUTE_CHUNK_0".to_string()
            }
            ExecutorStatsEnum::MemOpsExecuteChunk1(_stat_duration) => {
                "MEM_OPS_EXECUTE_CHUNK_1".to_string()
            }
            ExecutorStatsEnum::MemOpsExecuteChunk2(_stat_duration) => {
                "MEM_OPS_EXECUTE_CHUNK_2".to_string()
            }
            ExecutorStatsEnum::MemOpsExecuteChunk3(_stat_duration) => {
                "MEM_OPS_EXECUTE_CHUNK_3".to_string()
            }
            ExecutorStatsEnum::MemOpsExecuteChunk4(_stat_duration) => {
                "MEM_OPS_EXECUTE_CHUNK_4".to_string()
            }
            ExecutorStatsEnum::MemOpsExecuteChunk5(_stat_duration) => {
                "MEM_OPS_EXECUTE_CHUNK_5".to_string()
            }
            ExecutorStatsEnum::MemOpsExecuteChunk6(_stat_duration) => {
                "MEM_OPS_EXECUTE_CHUNK_6".to_string()
            }
            ExecutorStatsEnum::MemOpsExecuteChunk7(_stat_duration) => {
                "MEM_OPS_EXECUTE_CHUNK_7".to_string()
            }
            ExecutorStatsEnum::PlanGenerationMain(_stat_duration) => {
                "PLAN_GENERATION_MAIN".to_string()
            }
            ExecutorStatsEnum::PlanGenerationSecondary(_stat_duration) => {
                "PLAN_GENERATION_SECONDARY".to_string()
            }
            ExecutorStatsEnum::PlanGenerationMemOpWait(_stat_duration) => {
                "PLAN_GENERATION_MEM_OP_WAIT".to_string()
            }
            ExecutorStatsEnum::PlanGenerationMemOp(_stat_duration) => {
                "PLAN_GENERATION_MEM_OP".to_string()
            }
            ExecutorStatsEnum::ConfigureInstances(_stat_duration) => {
                "CONFIGURE_INSTANCES".to_string()
            }
            ExecutorStatsEnum::PreCalculateWitness(_stat_duration) => {
                "PRE_CALCULATE_WITNESS".to_string()
            }
            ExecutorStatsEnum::CalculateWitness(_stat_duration) => "CALCULATE_WITNESS".to_string(),
            ExecutorStatsEnum::End(_stat_duration) => "END".to_string(),
            ExecutorStatsEnum::Air(_stat_duration) => {
                panic!("ExecutorStats::stats_name() got an air stats");
            }
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
                ExecutorStatsEnum::ChunkPlayerMT(stat_duration)
                | ExecutorStatsEnum::AsmMtGeneration(stat_duration)
                | ExecutorStatsEnum::AsmRomHistogram(stat_duration)
                | ExecutorStatsEnum::AsmMemOps(stat_duration)
                | ExecutorStatsEnum::AsmWriteInput(stat_duration)
                | ExecutorStatsEnum::MemOpsChunkDone(stat_duration)
                | ExecutorStatsEnum::MemOpsProcessChunks(stat_duration)
                | ExecutorStatsEnum::MemOpsCollectPlans(stat_duration)
                | ExecutorStatsEnum::MemOpsCountPhase(stat_duration)
                | ExecutorStatsEnum::MemOpsPlanPhase(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk0(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk1(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk2(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk3(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk4(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk5(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk6(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk7(stat_duration)
                | ExecutorStatsEnum::PlanGenerationMain(stat_duration)
                | ExecutorStatsEnum::PlanGenerationSecondary(stat_duration)
                | ExecutorStatsEnum::PlanGenerationMemOpWait(stat_duration)
                | ExecutorStatsEnum::PlanGenerationMemOp(stat_duration)
                | ExecutorStatsEnum::ConfigureInstances(stat_duration)
                | ExecutorStatsEnum::PreCalculateWitness(stat_duration)
                | ExecutorStatsEnum::CalculateWitness(stat_duration)
                | ExecutorStatsEnum::End(stat_duration) => {
                    let name = ExecutorStats::stats_name(stat);
                    let start =
                        stat_duration.start_time.duration_since(start_time).as_nanos() as u64;
                    let duration = stat_duration.duration.as_nanos() as u64;
                    tasks.push(Task { name, start, duration });
                }
                ExecutorStatsEnum::MTChunkDone(_stat_duration) => {}
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

        // Sort by start time
        tasks.sort_by(|a, b| a.start.cmp(&b.start));

        // Save to stats.json

        // Convert to pretty-printed JSON
        let json = serde_json::to_string_pretty(&tasks).unwrap();

        // Write to file
        let json_file_name = format!("stats_{}.json", process::id());
        let _ = fs::write(&json_file_name, json);

        // Save to stats.csv

        // Create a CSV-formatted string with the tasks data
        let mut csv = String::new();
        for task in tasks {
            csv += &format!("{},{},{},\n", task.name, task.start, task.duration);
        }

        // Write to file
        let csv_file_name = format!("stats_{}.csv", process::id());
        let _ = fs::write(&csv_file_name, csv);

        tracing::info!("Statistics have been saved to {} and {}", json_file_name, csv_file_name);
    }

    /// Stores stats in JSON and CSV file formats
    pub fn print_stats(&self) {
        let start_time = self.start_time;
        let stats = &self.stats;

        println!("Collected a total of {} statistics", stats.len());
        for stat in stats.iter() {
            match stat {
                ExecutorStatsEnum::ChunkPlayerMT(stat_duration)
                | ExecutorStatsEnum::MTChunkDone(stat_duration)
                | ExecutorStatsEnum::AsmMtGeneration(stat_duration)
                | ExecutorStatsEnum::AsmRomHistogram(stat_duration)
                | ExecutorStatsEnum::AsmMemOps(stat_duration)
                | ExecutorStatsEnum::AsmWriteInput(stat_duration)
                | ExecutorStatsEnum::MemOpsChunkDone(stat_duration)
                | ExecutorStatsEnum::MemOpsProcessChunks(stat_duration)
                | ExecutorStatsEnum::MemOpsCollectPlans(stat_duration)
                | ExecutorStatsEnum::MemOpsCountPhase(stat_duration)
                | ExecutorStatsEnum::MemOpsPlanPhase(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk0(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk1(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk2(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk3(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk4(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk5(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk6(stat_duration)
                | ExecutorStatsEnum::MemOpsExecuteChunk7(stat_duration)
                | ExecutorStatsEnum::PlanGenerationMain(stat_duration)
                | ExecutorStatsEnum::PlanGenerationSecondary(stat_duration)
                | ExecutorStatsEnum::PlanGenerationMemOpWait(stat_duration)
                | ExecutorStatsEnum::PlanGenerationMemOp(stat_duration)
                | ExecutorStatsEnum::ConfigureInstances(stat_duration)
                | ExecutorStatsEnum::PreCalculateWitness(stat_duration)
                | ExecutorStatsEnum::CalculateWitness(stat_duration)
                | ExecutorStatsEnum::End(stat_duration) => {
                    let name = ExecutorStats::stats_name(stat);
                    let start =
                        stat_duration.start_time.duration_since(start_time).as_nanos() as u64;
                    let duration = stat_duration.duration.as_nanos() as u64;
                    println!("{name} start = {start} duration = {duration}");
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
                            "{name} start = {collect_start_time} duration = {collect_duration}"
                        );
                    }
                    if witness_duration > 0 {
                        let name = name.clone() + "_witness";
                        println!(
                            "{name} start = {witness_start_time} duration = {witness_duration}"
                        );
                    }
                }
            }
        }
    }
}
