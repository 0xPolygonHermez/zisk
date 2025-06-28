use std::sync::{atomic::AtomicBool, Arc};

use crate::{ServerConfig, ZiskBaseResponse, ZiskResponse};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ZiskStatusRequest;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ZiskStatus {
    Idle,
    Working,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZiskStatusResponse {
    #[serde(flatten)]
    pub base: ZiskBaseResponse,

    server_id: String,
    elf_file: String,
    uptime: String,
    status: ZiskStatus,
}

pub struct ZiskServiceStatusHandler;

impl ZiskServiceStatusHandler {
    pub fn handle(config: &ServerConfig, _payload: ZiskStatusRequest, is_busy: Arc<AtomicBool>) -> ZiskResponse {
        let uptime = config.launch_time.elapsed();

        ZiskResponse::ZiskStatusResponse(ZiskStatusResponse {
            base: ZiskBaseResponse {
                cmd: "status".to_string(),
                result: crate::ZiskCmdResult::Ok,
                code: crate::ZiskResultCode::Ok,
                msg: None,
            },
            server_id: config.server_id.to_string(),
            elf_file: config.elf.display().to_string(),
            uptime: format!("{:.2?}", uptime),
            status: if is_busy.load(std::sync::atomic::Ordering::SeqCst) {
                ZiskStatus::Working
            } else {
                ZiskStatus::Idle
            },
        })
    }
}
