use crate::ZiskResponse;

pub struct ZiskServiceShutdownHandler;

impl ZiskServiceShutdownHandler {
    pub fn handle() -> ZiskResponse {
        let msg = serde_json::json!({
            "info": "Shutting down server"
        });
        ZiskResponse::Ok { message: msg.to_string() }
    }
}
