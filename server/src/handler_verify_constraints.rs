use std::{path::PathBuf, sync::Arc, thread::JoinHandle};

use crate::{
    ServerConfig, ZiskBaseResponse, ZiskCmdResult, ZiskResponse, ZiskResultCode, ZiskService,
};
use colored::Colorize;
use fields::Goldilocks;
use proofman::ProofMan;
use proofman_common::DebugInfo;
use serde::{Deserialize, Serialize};
use zisk_common::{ExecutorStats, ZiskExecutionResult, ZiskLib};

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
        witness_lib: Arc<Box<dyn ZiskLib<Goldilocks>>>,
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
                let (result, mut _stats): (ZiskExecutionResult, ExecutorStats) =
                    witness_lib.get_execution_result().expect("Failed to get execution result");

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

                is_busy.store(false, std::sync::atomic::Ordering::SeqCst);
                ZiskService::print_waiting_message(&config);

                // Store the stats in stats.json
                #[cfg(feature = "stats")]
                {
                    let stats_id = _stats.next_id();
                    _stats.add_stat(0, stats_id, "END", 0, ExecutorStatsEvent::Mark);
                    _stats.store_stats();
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
