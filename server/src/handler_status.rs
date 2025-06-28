use crate::{ServerConfig, ZiskBaseResponse, ZiskResponse};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ZiskStatusRequest;

#[derive(Serialize, Deserialize, Debug)]
pub struct ZiskStatusResponse {
    #[serde(flatten)]
    pub base: ZiskBaseResponse,

    server_id: String,
    elf_file: String,
    uptime: String,
}

pub struct ZiskServiceStatusHandler;

impl ZiskServiceStatusHandler {
    pub fn handle(config: &ServerConfig, _payload: ZiskStatusRequest) -> ZiskResponse {
        let uptime = config.launch_time.elapsed();

        ZiskResponse::ZiskStatusResponse(ZiskStatusResponse {
            base: ZiskBaseResponse {
                cmd: "status".to_string(),
                status: crate::ZiskCmdStatus::Ok,
                code: crate::ZiskStatusCode::Ok,
                msg: None,
            },
            server_id: config.server_id.to_string(),
            elf_file: config.elf.display().to_string(),
            uptime: format!("{:.2?}", uptime),
        })
    }
}
