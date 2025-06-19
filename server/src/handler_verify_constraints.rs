use std::path::PathBuf;

use crate::{ServerConfig, ZiskResponse};
use colored::Colorize;
use executor::{Stats, ZiskExecutionResult};
use fields::Goldilocks;
use proofman::ProofMan;
use proofman_common::DebugInfo;
use serde::{Deserialize, Serialize};
use witness::WitnessLibrary;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ZiskVerifyConstraintsRequest {
    pub input: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZiskVerifyConstraintsResponse {
    pub success: bool,
    pub details: String,
}

pub struct ZiskServiceVerifyConstraintsHandler;

impl ZiskServiceVerifyConstraintsHandler {
    pub fn handle(
        config: &ServerConfig,
        request: ZiskVerifyConstraintsRequest,
        proofman: &ProofMan<Goldilocks>,
        witness_lib: &mut dyn WitnessLibrary<Goldilocks>,
        debug_info: &DebugInfo,
    ) -> ZiskResponse {
        let uptime = config.launch_time.elapsed();

        let status = serde_json::json!({
            "server_id": config.server_id.to_string(),
            "elf_file": config.elf.display().to_string(),
            "uptime": format!("{:.2?}", uptime),
            "command:": "VerifyConstraints",
            "payload:": {
                "input": request.input.display().to_string(),
            },
        });

        let start = std::time::Instant::now();

        proofman
            .verify_proof_constraints_from_lib(Some(request.input), debug_info)
            .map_err(|e| anyhow::anyhow!("Error verifying proof: {}", e))
            .expect("Failed to generate proof");

        let elapsed = start.elapsed();

        let result: (ZiskExecutionResult, Vec<(usize, usize, Stats)>) = *witness_lib
            .get_execution_result()
            .ok_or_else(|| anyhow::anyhow!("No execution result found"))
            .expect("Failed to get execution result")
            .downcast::<(ZiskExecutionResult, Vec<(usize, usize, Stats)>)>()
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
            result.0.executed_steps
        );

        ZiskResponse::Ok { message: status.to_string() }
    }
}
