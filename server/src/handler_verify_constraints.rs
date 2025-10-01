use std::{path::PathBuf, sync::Arc, thread::JoinHandle};

use crate::{
    ServerConfig, ZiskBaseResponse, ZiskCmdResult, ZiskResponse, ZiskResultCode, ZiskService,
};
use colored::Colorize;
use executor::{Stats, ZiskExecutionResult};
use fields::Goldilocks;
use proofman::ProofMan;
use proofman_common::DebugInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use witness::WitnessLibrary;
use zisk_common::ExecutorStats;

#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ZiskVerifyConstraintsRequest {
    pub input: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZiskVerifyConstraintsResponse {
    #[serde(flatten)]
    pub base: ZiskBaseResponse,

    server_id: String,
    elf_file: String,
    input: String,
}

pub struct ZiskServiceVerifyConstraintsHandler;

impl ZiskServiceVerifyConstraintsHandler {
    pub fn handle(
        config: Arc<ServerConfig>,
        request: ZiskVerifyConstraintsRequest,
        // It is important to keep the witness_lib declaration before the proofman declaration
        // to ensure that the witness library is dropped before the proofman.
        witness_lib: Arc<dyn WitnessLibrary<Goldilocks> + Send + Sync>,
        proofman: Arc<ProofMan<Goldilocks>>,
        is_busy: Arc<std::sync::atomic::AtomicBool>,
        debug_info: Arc<DebugInfo>,
    ) -> (ZiskResponse, Option<JoinHandle<()>>) {
        is_busy.store(true, std::sync::atomic::Ordering::SeqCst);

        let handle = std::thread::spawn({
            let request_input = request.input.clone();
            let config = config.clone();
            move || {
                let start = std::time::Instant::now();

                proofman
                    .verify_proof_constraints_from_lib(Some(request_input), &debug_info, false)
                    .map_err(|e| anyhow::anyhow!("Error verifying proof: {}", e))
                    .expect("Failed to generate proof");
                proofman.set_barrier();
                let elapsed = start.elapsed();

                #[allow(clippy::type_complexity)]
                let result: (
                    Arc<Mutex<ZiskExecutionResult>>,
                    Arc<Mutex<ExecutorStats>>,
                    Arc<Mutex<HashMap<usize, Stats>>>,
                ) = *witness_lib
                    .get_execution_result()
                    .ok_or_else(|| anyhow::anyhow!("No execution result found"))
                    .expect("Failed to get execution result")
                    .downcast::<(
                        Arc<Mutex<ZiskExecutionResult>>,
                        Arc<Mutex<ExecutorStats>>,
                        Arc<Mutex<HashMap<usize, Stats>>>,
                    )>()
                    .map_err(|_| anyhow::anyhow!("Failed to downcast execution result"))
                    .expect("Failed to downcast execution result");

                println!();
                tracing::info!(
                    "{}",
                    "--- VERIFY CONSTRAINTS SUMMARY ------------------------".bright_green().bold()
                );
                tracing::info!("    ► Statistics");
                tracing::info!(
                    "      time: {} seconds, steps: {}",
                    elapsed.as_secs_f32(),
                    result.0.lock().unwrap().executed_steps
                );

                is_busy.store(false, std::sync::atomic::Ordering::SeqCst);
                ZiskService::print_waiting_message(&config);

                // Store the stats in stats.json
                #[cfg(feature = "stats")]
                {
                    let stats = result.1;
                    let stats_id = stats.lock().unwrap().get_id();
                    stats.lock().unwrap().add_stat(0, stats_id, "END", 0, ExecutorStatsEvent::Mark);
                    stats.lock().unwrap().store_stats();
                }
            }
        });

        (
            ZiskResponse::ZiskVerifyConstraintsResponse(ZiskVerifyConstraintsResponse {
                base: ZiskBaseResponse {
                    cmd: "verify_constraints".to_string(),
                    result: ZiskCmdResult::InProgress,
                    code: ZiskResultCode::Ok,
                    msg: None,
                    node: config.asm_runner_options.world_rank,
                },
                server_id: config.server_id.to_string(),
                elf_file: config.elf.display().to_string(),
                input: request.input.display().to_string(),
            }),
            Some(handle),
        )
    }
    pub fn process_handle(
        request: ZiskVerifyConstraintsRequest,
        proofman: Arc<ProofMan<Goldilocks>>,
        debug_info: Arc<DebugInfo>,
    ) {
        proofman
            .verify_proof_constraints_from_lib(Some(request.input), &debug_info, false)
            .map_err(|e| anyhow::anyhow!("Error verifying proof: {}", e))
            .expect("Failed to generate proof");
        proofman.set_barrier();
    }
}
