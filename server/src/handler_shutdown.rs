use asm_runner::AsmServices;

use crate::{ServerConfig, ZiskResponse};

pub struct ZiskServiceShutdownHandler;

impl ZiskServiceShutdownHandler {
    pub fn handle(asm_services: &AsmServices, config: &ServerConfig) -> ZiskResponse {
        let msg = serde_json::json!({
            "info": "Shutting down server"
        });
        tracing::info!(
            "<<< [{}] Shutting down ASM microservices.",
            config.asm_runner_options.world_rank
        );
        asm_services.stop_asm_services().expect("Failed to stop ASM services");
        ZiskResponse::Ok { message: msg.to_string() }
    }
}
