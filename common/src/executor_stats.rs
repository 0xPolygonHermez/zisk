use std::{
    collections::HashMap,
    fs, process,
    sync::{Arc, Mutex},
    time::Instant,
};

use serde::{Deserialize, Serialize};
#[cfg(feature = "stats")]
use zisk_pil::*;

use crate::Stats;

#[derive(Debug, Clone)]
pub enum ExecutorStatsEvent {
    Begin,
    End,
    Mark,
}

#[derive(Debug, Clone)]
struct ExecutorStatsEntry {
    parent_id: u64,
    id: u64,
    name: &'static str,
    index: usize,
    event: ExecutorStatsEvent,
    timestamp: Instant,
}

#[derive(Debug, Clone)]
pub struct ExecutorStats {
    start_time: Instant,
    last_id: u64,
    stats: Vec<ExecutorStatsEntry>,
    pub witness_stats: HashMap<usize, Stats>,
}

impl Default for ExecutorStats {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutorStats {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            last_id: 0,
            stats: Vec::new(),
            witness_stats: HashMap::new(),
        }
    }

    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.last_id = 0;
        self.stats.clear();
        self.witness_stats.clear();
    }

    pub fn add_stat(
        &mut self,
        parent_id: u64,
        id: u64,
        name: &'static str,
        index: usize,
        event: ExecutorStatsEvent,
    ) {
        let stat =
            ExecutorStatsEntry { parent_id, id, name, index, event, timestamp: Instant::now() };
        self.stats.push(stat);
    }

    pub fn set_start_time(&mut self, start_time: Instant) {
        self.start_time = start_time;
    }

    pub fn next_id(&mut self) -> u64 {
        self.last_id += 1;
        self.last_id
    }

    #[cfg(feature = "stats")]
    fn _air_name(_airgroup_id: usize, air_id: usize) -> String {
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

    /// Stores stats in JSON and CSV file formats
    pub fn store_stats(&self) {
        #[derive(Serialize, Deserialize, Debug)]
        struct Task {
            parent_id: u64,
            id: u64,
            name: String,
            index: u64,
            event: String,
            timestamp: u64,
        }
        let mut tasks: Vec<Task> = Vec::new();

        for stat in &self.stats {
            let task = Task {
                parent_id: stat.parent_id,
                id: stat.id,
                name: stat.name.to_string(),
                index: stat.index as u64,
                event: match stat.event {
                    ExecutorStatsEvent::Begin => "Begin".to_string(),
                    ExecutorStatsEvent::End => "End".to_string(),
                    ExecutorStatsEvent::Mark => "Mark".to_string(),
                },
                timestamp: stat.timestamp.duration_since(self.start_time).as_nanos() as u64,
            };
            tasks.push(task);
        }

        // Order by timestamp
        tasks.sort_by_key(|task| task.timestamp);

        tracing::info!("Collected a total of {} statistics", tasks.len());

        // Save to stats.json
        /////////////////////

        // Convert to pretty-printed JSON
        let json = serde_json::to_string_pretty(&tasks).unwrap();

        // Write to file
        let json_file_name = format!("stats_{}.json", process::id());
        let _ = fs::write(&json_file_name, json);

        // Save to stats.csv
        ////////////////////

        // Create a CSV-formatted string with the tasks data
        let mut csv = String::new();
        for task in tasks {
            csv += &format!(
                "{},{},{},{},{},{}\n",
                task.parent_id, task.id, task.name, task.index, task.event, task.timestamp
            );
        }

        // Write to file
        let csv_file_name = format!("stats_{}.csv", process::id());
        let _ = fs::write(&csv_file_name, csv);

        tracing::info!("Statistics have been saved to {} and {}", json_file_name, csv_file_name);
    }

    /// Prints stats
    pub fn print_stats(&self) {
        println!("Collected a total of {} statistics", self.stats.len());
        for stat in &self.stats {
            println!(
                "parent_id={} id={} name={} index={} event={:?} timestamp={}",
                stat.parent_id,
                stat.id,
                stat.name,
                stat.index,
                stat.event,
                stat.timestamp.duration_since(self.start_time).as_nanos() as u64
            );
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ExecutorStatsHandle {
    inner: Arc<Mutex<ExecutorStats>>,
}

impl ExecutorStatsHandle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn reset(&self) {
        self.inner.lock().unwrap().reset();
    }

    pub fn add_stat(
        &self,
        parent_id: u64,
        id: u64,
        name: &'static str,
        index: usize,
        event: ExecutorStatsEvent,
    ) {
        self.inner.lock().unwrap().add_stat(parent_id, id, name, index, event);
    }

    pub fn set_start_time(&self, start_time: Instant) {
        self.inner.lock().unwrap().set_start_time(start_time);
    }

    pub fn next_id(&self) -> u64 {
        self.inner.lock().unwrap().next_id()
    }

    #[cfg(feature = "stats")]
    pub fn _air_name(&self, airgroup_id: usize, air_id: usize) -> String {
        ExecutorStats::_air_name(airgroup_id, air_id)
    }

    pub fn store_stats(&self) {
        self.inner.lock().unwrap().store_stats();
    }

    pub fn print_stats(&self) {
        self.inner.lock().unwrap().print_stats();
    }

    pub fn get_inner(&self) -> ExecutorStats {
        self.inner.lock().unwrap().clone()
    }

    pub fn insert_witness_stats(&self, airgroup_id: usize, stats: Stats) {
        self.inner.lock().unwrap().witness_stats.insert(airgroup_id, stats);
    }

    pub fn set_witness_duration(&self, airgroup_id: usize, duration: u128) {
        if let Some(stats) = self.inner.lock().unwrap().witness_stats.get_mut(&airgroup_id) {
            stats.witness_duration = duration;
        }
    }
}
