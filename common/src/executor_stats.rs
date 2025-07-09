use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

pub enum ExecutorStats {
    GenerateMT(StatsDuration),
    ChunkPlayerMT(StatsDuration),
    Air(ExecuteStatsAir),
}

pub struct StatsDuration {
    pub start_time: Instant,
    pub duration: Duration,
}

pub struct ExecuteStatsAir {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub stats_collect: StatsDuration,
    pub stats_witness: StatsDuration,
    pub num_chunks: usize,
}

pub struct ExecuteStats {
    pub stats: Mutex<Vec<ExecutorStats>>,
}

impl ExecuteStats {
    pub fn new() -> Self {
        Self { stats: Mutex::new(Vec::new()) }
    }

    pub fn add_stats(&self, stat: ExecutorStats) {
        let mut stats = self.stats.lock().unwrap();
        stats.push(stat);
    }
}
