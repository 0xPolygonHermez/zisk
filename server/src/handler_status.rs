use crate::{ServerConfig, ZiskResponse};

pub struct ZiskServiceStatusHandler;

impl ZiskServiceStatusHandler {
    pub fn handle(config: &ServerConfig) -> ZiskResponse {
        let uptime = config.launch_time.elapsed();
        let status = serde_json::json!({
            "server_id": config.server_id.to_string(),
            "elf_file": config.elf.display().to_string(),
            "uptime": format!("{:.2?}", uptime)
        });
        ZiskResponse::Ok { message: status.to_string() }
    }
}
