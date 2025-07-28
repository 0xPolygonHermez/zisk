use bytemuck::cast_slice;
use colored::Colorize;
use executor::ZiskExecutionResult;
use fields::Goldilocks;
use proofman::ProofMan;
use proofman_common::ProofOptions;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::{fs::File, path::PathBuf};
use witness::WitnessLibrary;
use zisk_common::{ExecutorStats, ProofLog};

use crate::{
    ServerConfig, ZiskBaseResponse, ZiskCmdResult, ZiskResponse, ZiskResultCode, ZiskService,
};

#[cfg(feature = "stats")]
use std::time::Duration;
#[cfg(feature = "stats")]
use std::time::Instant;
#[cfg(feature = "stats")]
use zisk_common::{ExecutorStatsDuration, ExecutorStatsEnum};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ZiskProveRequest {
    pub input: PathBuf,
    pub aggregation: bool,
    pub final_snark: bool,
    pub verify_proofs: bool,
    pub minimal_memory: bool,
    pub folder: PathBuf,
    pub prefix: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZiskProveResponse {
    #[serde(flatten)]
    pub base: ZiskBaseResponse,

    server_id: String,
    elf_file: String,
    input: String,
}
pub struct ZiskServiceProveHandler;

impl ZiskServiceProveHandler {
    pub fn handle(
        config: Arc<ServerConfig>,
        request: ZiskProveRequest,
        // It is important to keep the witness_lib declaration before the proofman declaration
        // to ensure that the witness library is dropped before the proofman.
        witness_lib: Arc<dyn WitnessLibrary<Goldilocks> + Send + Sync>,
        proofman: Arc<ProofMan<Goldilocks>>,
        is_busy: Arc<std::sync::atomic::AtomicBool>,
    ) -> (ZiskResponse, Option<JoinHandle<()>>) {
        is_busy.store(true, std::sync::atomic::Ordering::SeqCst);

        let handle = std::thread::spawn({
            let request_input = request.input.clone();
            let config = config.clone();
            move || {
                let start = std::time::Instant::now();

                let (proof_id, vadcop_final_proof) = proofman
                    .generate_proof_from_lib(
                        Some(request_input),
                        ProofOptions::new(
                            false,
                            request.aggregation,
                            request.final_snark,
                            request.verify_proofs,
                            request.minimal_memory,
                            false,
                            request.folder.clone(),
                        ),
                    )
                    .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))
                    .expect("Failed to generate proof");

                let elapsed = start.elapsed();

                if proofman.get_rank() == Some(0) || proofman.get_rank().is_none() {
                    let (result, _stats): (ZiskExecutionResult, Arc<Mutex<ExecutorStats>>) =
                        *witness_lib
                            .get_execution_result()
                            .ok_or_else(|| anyhow::anyhow!("No execution result found"))
                            .expect("Failed to get execution result")
                            .downcast::<(ZiskExecutionResult, Arc<Mutex<ExecutorStats>>)>()
                            .map_err(|_| anyhow::anyhow!("Failed to downcast execution result"))
                            .expect("Failed to downcast execution result");

                    let elapsed = elapsed.as_secs_f64();
                    tracing::info!("");
                    tracing::info!(
                        "{}",
                        "--- PROVE SUMMARY ------------------------".bright_green().bold()
                    );
                    if let Some(proof_id) = &proof_id {
                        tracing::info!("      Proof ID: {}", proof_id);
                    }
                    tracing::info!("    ► Statistics");
                    tracing::info!(
                        "      time: {} seconds, steps: {}",
                        elapsed,
                        result.executed_steps
                    );

                    // Store the stats in stats.json
                    #[cfg(feature = "stats")]
                    {
                        _stats.lock().unwrap().add_stat(ExecutorStatsEnum::End(
                            ExecutorStatsDuration {
                                start_time: Instant::now(),
                                duration: Duration::new(0, 1),
                            },
                        ));
                        _stats.lock().unwrap().store_stats();
                    }

                    if let Some(proof_id) = proof_id {
                        let logs = ProofLog::new(result.executed_steps, proof_id, elapsed);
                        let log_path =
                            request.folder.join(format!("{}-result.json", request.prefix));
                        println!("Writing proof log to: {}", log_path.display());
                        ProofLog::write_json_log(&log_path, &logs)
                            .map_err(|e| anyhow::anyhow!("Error generating log: {}", e))
                            .expect("Failed to generate proof");
                        // Save the vadcop final proof
                        let proof_path = request
                            .folder
                            .join(format!("{}-vadcop_final_proof.bin", request.prefix));
                        // write a Vec<u64> to a bin file stored in output_file_path
                        let mut file = File::create(proof_path).expect("Error while creating file");
                        file.write_all(cast_slice(&vadcop_final_proof.unwrap()))
                            .expect("Error while writing to file");
                    }
                }
                is_busy.store(false, std::sync::atomic::Ordering::SeqCst);
                ZiskService::print_waiting_message(&config);
            }
        });

        (
            ZiskResponse::ZiskProveResponse(ZiskProveResponse {
                base: ZiskBaseResponse {
                    cmd: "prove".to_string(),
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
}
