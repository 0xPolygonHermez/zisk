use asm_runner::AsmServices;

use serde::{Deserialize, Serialize};

use crate::{ServerConfig, ZiskBaseResponse, ZiskResponse};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ZiskShutdownRequest;

#[derive(Serialize, Deserialize, Debug)]
pub struct ZiskShutdownResponse {
    #[serde(flatten)]
    pub base: ZiskBaseResponse,
}

pub struct ZiskServiceShutdownHandler;

impl ZiskServiceShutdownHandler {
    pub fn handle(
        config: &ServerConfig,
        _payload: ZiskShutdownRequest,
        asm_services: &AsmServices,
    ) -> ZiskResponse {
        tracing::info!(
            "<<< [{}] Shutting down ASM microservices.",
            config.asm_runner_options.world_rank
        );

        let shutdown_result = asm_services.stop_asm_services();

        if let Err(e) = shutdown_result {
            tracing::error!("Failed to stop ASM services: {}", e);
            return ZiskResponse::ZiskShutdownResponse(ZiskShutdownResponse {
                base: ZiskBaseResponse {
                    cmd: "shutdown".to_string(),
                    status: crate::ZiskCmdStatus::Error,
                    code: crate::ZiskStatusCode::Error,
                    msg: Some(format!("Failed to stop ASM services: {}", e)),
                },
            });
        }

        ZiskResponse::ZiskShutdownResponse(ZiskShutdownResponse {
            base: ZiskBaseResponse {
                cmd: "shutdown".to_string(),
                status: crate::ZiskCmdStatus::Ok,
                code: crate::ZiskStatusCode::Ok,
                msg: None,
            },
        })
    }
}
