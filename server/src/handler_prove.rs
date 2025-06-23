use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{ServerConfig, ZiskResponse};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ZiskProveRequest {
    pub input: PathBuf,
    pub aggregation: bool,
    pub final_snark: bool,
    pub verify_proofs: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZiskProveResponse {
    pub success: bool,
    pub details: String,
}

pub struct ZiskServiceProveHandler;

impl ZiskServiceProveHandler {
    pub fn handle(config: &ServerConfig, payload: ZiskProveRequest) -> ZiskResponse {
        let uptime = config.launch_time.elapsed();
        let status = serde_json::json!({
            "server_id": config.server_id.to_string(),
            "elf_file": config.elf.display().to_string(),
            "uptime": format!("{:.2?}", uptime),
            "command:": "prove",
            "payload:": {
                "input": payload.input.display().to_string(),
                "aggregation": payload.aggregation,
                "final_snark": payload.final_snark,
                "verify_proofs": payload.verify_proofs,
            },
        });

        ZiskResponse::Ok { message: status.to_string() }
    }
}
