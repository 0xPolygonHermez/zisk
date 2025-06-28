use bytemuck::cast_slice;
use colored::Colorize;
use executor::{Stats, ZiskExecutionResult};
use fields::Goldilocks;
use proofman::ProofMan;
use proofman_common::ProofOptions;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{fs::File, path::PathBuf};
use witness::WitnessLibrary;
use zisk_common::ProofLog;

use crate::{ServerConfig, ZiskBaseResponse, ZiskResponse};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ZiskProveRequest {
    pub input: PathBuf,
    pub aggregation: bool,
    pub final_snark: bool,
    pub verify_proofs: bool,
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
    duration_ms: u64,
}

pub struct ZiskServiceProveHandler;

impl ZiskServiceProveHandler {
    pub fn handle(
        config: &ServerConfig,
        request: ZiskProveRequest,
        proofman: &ProofMan<Goldilocks>,
        witness_lib: &mut dyn WitnessLibrary<Goldilocks>,
    ) -> ZiskResponse {
        let start = std::time::Instant::now();

        let request_input = request.input.clone();
        let (proof_id, vadcop_final_proof) = proofman
            .generate_proof_from_lib(
                Some(request.input),
                ProofOptions::new(
                    false,
                    request.aggregation,
                    request.final_snark,
                    request.verify_proofs,
                    false,
                    request.folder.clone(),
                ),
            )
            .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))
            .expect("Failed to generate proof");

        let elapsed = start.elapsed();

        if proofman.get_rank() == Some(0) || proofman.get_rank().is_none() {
            let (result, _): (ZiskExecutionResult, Vec<(usize, usize, Stats)>) = *witness_lib
                .get_execution_result()
                .ok_or_else(|| anyhow::anyhow!("No execution result found"))
                .expect("Failed to get execution result")
                .downcast::<(ZiskExecutionResult, Vec<(usize, usize, Stats)>)>()
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
            tracing::info!("      time: {} seconds, steps: {}", elapsed, result.executed_steps);

            if let Some(proof_id) = proof_id {
                let logs = ProofLog::new(result.executed_steps, proof_id, elapsed);
                let log_path = request.folder.join(format!("{}-result.json", request.prefix));
                println!("Writing proof log to: {}", log_path.display());
                ProofLog::write_json_log(&log_path, &logs)
                    .map_err(|e| anyhow::anyhow!("Error generating log: {}", e))
                    .expect("Failed to generate proof");
                // Save the vadcop final proof
                let proof_path =
                    request.folder.join(format!("{}-vadcop_final_proof.bin", request.prefix));
                // write a Vec<u64> to a bin file stored in output_file_path
                let mut file = File::create(proof_path).expect("Error while creating file");
                file.write_all(cast_slice(&vadcop_final_proof.unwrap()))
                    .expect("Error while writing to file");
            }
        }

        ZiskResponse::ZiskProveResponse(ZiskProveResponse {
            base: ZiskBaseResponse {
                cmd: "prove".to_string(),
                status: crate::ZiskCmdStatus::Ok,
                code: crate::ZiskStatusCode::Ok,
                msg: None,
            },
            server_id: config.server_id.to_string(),
            elf_file: config.elf.display().to_string(),
            input: request_input.display().to_string(),
            duration_ms: elapsed.as_millis() as u64,
        })
    }
}
