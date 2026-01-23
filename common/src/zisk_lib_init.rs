use std::{path::PathBuf, time::Instant};

use fields::PrimeField64;
use proofman_common::VerboseMode;
use witness::WitnessLibrary;

use crate::{
    io::{StreamSource, ZiskStdin},
    ExecutorStats,
};

use anyhow::Result;

#[derive(Debug, Default, Clone)]
pub struct ZiskExecutionResult {
    pub steps: u64,
}

impl ZiskExecutionResult {
    pub fn new(steps: u64) -> Self {
        Self { steps }
    }
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub airgroup_id: usize,
    pub air_id: usize,
    /// Collect start time
    pub collect_start_time: Instant,
    /// Collect duration in microseconds
    pub collect_duration: u64,
    /// Witness start time
    pub witness_start_time: Instant,
    /// Witness duration in microseconds
    pub witness_duration: u128,
    /// Number of chunks
    pub num_chunks: usize,
}

/// Extension trait that provides execution result access without Any boxing
pub trait ZiskWitnessLibrary<F: PrimeField64> {
    fn set_stdin(&self, stdin: ZiskStdin);
    fn set_hints_stream(&self, stream: StreamSource) -> Result<()>;
    fn execution_result(&self) -> Option<(ZiskExecutionResult, ExecutorStats)>;
}

// SUpertrait for ZiskWitnessLibrary and WitnessLibrary
pub trait ZiskLib<F: PrimeField64>:
    WitnessLibrary<F> + ZiskWitnessLibrary<F> + Send + Sync
{
}

pub type ZiskLibInitFn<F> = fn(
    VerboseMode,
    PathBuf,         // Rom path
    Option<PathBuf>, // Asm path
    Option<PathBuf>, // Asm ROM path
    Option<u16>,     // Base port for the ASM microservices
    bool,            // Unlock_mapped_memory
    bool,            // Shared_tables
) -> Result<Box<dyn ZiskLib<F>>, Box<dyn std::error::Error>>;
