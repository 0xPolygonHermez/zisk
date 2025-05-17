use colored::Colorize;
use executor::ZiskExecutionResult;
use libloading::{Library, Symbol};
use p3_goldilocks::Goldilocks;
use proofman::ProofMan;
use proofman_common::ProofOptions;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;
use zisk_common::ZiskLibInitFn;

use crate::{Response, ServerConfig};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VerifyConstraintsRequest {
    pub input: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyConstraintsResponse {
    pub success: bool,
    pub details: String,
}

pub fn handle_verify_constraints(
    config: &ServerConfig,
    payload: VerifyConstraintsRequest,
) -> Response {
    let uptime = config.launch_time.elapsed();
    let status = serde_json::json!({
        "server_id": config.server_id.to_string(),
        "elf_file": config.elf.display().to_string(),
        "uptime": format!("{:.2?}", uptime),
        "command:": "VerifyConstraints",
        "payload:": {
            "input": payload.input.display().to_string(),
        },
    });

    let start = std::time::Instant::now();

    let mut witness_lib;
    let library =
        unsafe { Library::new(config.witness_lib.clone()).expect("Failed to load library") };
    let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
        unsafe { library.get(b"init_library").expect("Failed to get symbol") };
    witness_lib = witness_lib_constructor(
        config.verbose.into(),
        config.elf.clone(),
        config.asm.clone(),
        config.asm_rom.clone(),
        Some(payload.input),
        config.sha256f_script.clone(),
    )
    .expect("Failed to initialize witness library");

    ProofMan::<Goldilocks>::verify_proof_constraints_from_lib(
        &mut *witness_lib,
        config.proving_key.clone(),
        PathBuf::new(),
        config.custom_commits_map.clone(),
        ProofOptions::new(true, config.verbose.into(), false, false, false, config.debug.clone()),
    )
    .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))
    .expect("Failed to generate proof");

    let elapsed = start.elapsed();

    let result: ZiskExecutionResult = *witness_lib
        .get_execution_result()
        .ok_or_else(|| anyhow::anyhow!("No execution result found"))
        .expect("Failed to get execution result")
        .downcast::<ZiskExecutionResult>()
        .map_err(|_| anyhow::anyhow!("Failed to downcast execution result"))
        .expect("Failed to downcast execution result");

    println!();
    info!(
        "{}",
        "    Zisk: --- VERIFY CONSTRAINTS SUMMARY ------------------------".bright_green().bold()
    );
    info!("              ► Statistics");
    info!(
        "                time: {} seconds, steps: {}",
        elapsed.as_secs_f32(),
        result.executed_steps
    );

    Response::Ok { message: status.to_string() }
}
