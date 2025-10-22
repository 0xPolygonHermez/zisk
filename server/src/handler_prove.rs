use bytemuck::cast_slice;
use colored::Colorize;
use fields::Goldilocks;
use proofman::ProofMan;
use proofman::{ProofInfo, ProvePhase, ProvePhaseInputs, ProvePhaseResult};
use proofman_common::ProofOptions;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{fs::File, path::PathBuf};
use zisk_common::{ExecutorStats, ProofLog, ZiskExecutionResult, ZiskLib};
use zstd::stream::write::Encoder;

use crate::{
    ServerConfig, ZiskBaseResponse, ZiskCmdResult, ZiskResponse, ZiskResultCode, ZiskService,
};

#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;

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
        witness_lib: Arc<Box<dyn ZiskLib<Goldilocks>>>,
        proofman: Arc<ProofMan<Goldilocks>>,
        is_busy: Arc<std::sync::atomic::AtomicBool>,
    ) -> (ZiskResponse, Option<JoinHandle<()>>) {
        is_busy.store(true, std::sync::atomic::Ordering::SeqCst);

        let handle = std::thread::spawn({
            let request_input = request.input.clone();
            let config = config.clone();
            move || {
                let start = std::time::Instant::now();

                let result = proofman
                    .generate_proof_from_lib(
                        ProvePhaseInputs::Full(ProofInfo::new(Some(request_input), 1, vec![0], 0)),
                        ProofOptions::new(
                            false,
                            request.aggregation,
                            request.final_snark,
                            request.verify_proofs,
                            request.minimal_memory,
                            false,
                            request.folder.clone(),
                        ),
                        ProvePhase::Full,
                    )
                    .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))
                    .expect("Failed to generate proof");

                let (proof_id, vadcop_final_proof) =
                    if let ProvePhaseResult::Full(proof_id, vadcop_final_proof) = result {
                        (proof_id, vadcop_final_proof)
                    } else {
                        (None, None)
                    };

                let elapsed = start.elapsed();

                if proofman.rank().unwrap() == 0 {
                    #[allow(clippy::type_complexity)]
                    let (result, mut _stats): (
                        ZiskExecutionResult,
                        ExecutorStats,
                    ) = witness_lib.get_execution_result().expect("Failed to get execution result");

                    proofman.set_barrier();
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
                        let stats_id = _stats.next_id();
                        _stats.add_stat(0, stats_id, "END", 0, ExecutorStatsEvent::Mark);
                        _stats.store_stats();
                    }

                    if let Some(proof_id) = proof_id {
                        let logs = ProofLog::new(result.executed_steps, proof_id, elapsed);
                        let log_path =
                            request.folder.join(format!("{}-result.json", request.prefix));
                        println!("Writing proof log to: {}", log_path.display());
                        ProofLog::write_json_log(&log_path, &logs)
                            .map_err(|e| anyhow::anyhow!("Error generating log: {}", e))
                            .expect("Failed to generate proof");
                        // Save the uncompressed vadcop final proof
                        let output_file_path = request
                            .folder
                            .join(format!("{}-vadcop_final_proof.bin", request.prefix));

                        let vadcop_proof = vadcop_final_proof.unwrap();
                        let proof_data = cast_slice(&vadcop_proof);
                        let mut file =
                            File::create(&output_file_path).expect("Error while creating file");
                        file.write_all(proof_data).expect("Error while writing to file");

                        // Save the compressed vadcop final proof using zstd (fastest compression level)
                        let compressed_output_path = request
                            .folder
                            .join(format!("{}-vadcop_final_proof.compressed.bin", request.prefix));
                        let compressed_file = File::create(&compressed_output_path).unwrap();
                        let mut encoder = Encoder::new(compressed_file, 1).unwrap();
                        encoder.write_all(proof_data).unwrap();
                        encoder.finish().unwrap();

                        let original_size = vadcop_proof.len() * 8;
                        let compressed_size =
                            std::fs::metadata(&compressed_output_path).unwrap().len();
                        let compression_ratio = compressed_size as f64 / original_size as f64;

                        println!("Vadcop final proof saved:");
                        println!("  Original: {} bytes", original_size);
                        println!(
                            "  Compressed: {} bytes (ratio: {:.2}x)",
                            compressed_size, compression_ratio
                        );
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
    pub fn process_handle(request: ZiskProveRequest, proofman: Arc<ProofMan<Goldilocks>>) {
        proofman
            .generate_proof_from_lib(
                ProvePhaseInputs::Full(ProofInfo::new(Some(request.input), 1, vec![0], 0)),
                ProofOptions::new(
                    false,
                    request.aggregation,
                    request.final_snark,
                    request.verify_proofs,
                    request.minimal_memory,
                    false,
                    request.folder.clone(),
                ),
                ProvePhase::Full,
            )
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))
            .expect("Failed to generate proof");
        proofman.set_barrier();
    }
}
