use std::thread::JoinHandle;

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
        asm_services: Option<&AsmServices>,
    ) -> (ZiskResponse, Option<JoinHandle<()>>) {
        tracing::info!(
            "<<< [{}] Shutting down ASM microservices.",
            config.asm_runner_options.world_rank
        );

        if let Some(asm_services) = asm_services {
            let shutdown_result = asm_services.stop_asm_services();

            if let Err(e) = shutdown_result {
                tracing::error!("Failed to stop ASM services: {}", e);
                return (
                    ZiskResponse::ZiskShutdownResponse(ZiskShutdownResponse {
                        base: ZiskBaseResponse {
                            cmd: "shutdown".to_string(),
                            result: crate::ZiskCmdResult::Error,
                            code: crate::ZiskResultCode::Error,
                            msg: Some(format!("Failed to stop ASM services: {e}")),
                            node: config.asm_runner_options.world_rank,
                        },
                    }),
                    None,
                );
            }
        }

        (
            ZiskResponse::ZiskShutdownResponse(ZiskShutdownResponse {
                base: ZiskBaseResponse {
                    cmd: "shutdown".to_string(),
                    result: crate::ZiskCmdResult::Ok,
                    code: crate::ZiskResultCode::Ok,
                    msg: None,
                    node: config.asm_runner_options.world_rank,
                },
            }),
            None,
        )
    }
}
