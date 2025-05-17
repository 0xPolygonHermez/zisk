use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    sync::Arc,
};

use crate::{
    handle_prove, handle_verify_constraints, ProveRequest, ServerConfig, VerifyConstraintsRequest,
};
use serde::{Deserialize, Serialize};
use zisk_common::info_file;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "command", rename_all = "lowercase")]
pub enum Request {
    Status,
    Shutdown,
    Prove {
        #[serde(flatten)]
        payload: ProveRequest,
    },
    VerifyConstraints {
        #[serde(flatten)]
        payload: VerifyConstraintsRequest,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Response {
    Ok { message: String },
    Error { message: String },
}

pub fn handle_client(mut stream: TcpStream, config: Arc<ServerConfig>) -> std::io::Result<bool> {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();

    reader.read_line(&mut line)?;

    let request: Request = match serde_json::from_str(&line) {
        Ok(req) => req,
        Err(e) => {
            let response = Response::Error { message: format!("Invalid JSON: {}", e) };
            send_json(&mut stream, &response)?;
            return Ok(false);
        }
    };

    info_file!("Received request: {:?}", request);

    let mut must_shutdown = false;
    let response = match request {
        Request::Status => handle_status(config.as_ref()),
        Request::Shutdown => {
            must_shutdown = true;
            handle_shutdown()
        }
        Request::VerifyConstraints { payload } => {
            handle_verify_constraints(config.as_ref(), payload)
        }
        Request::Prove { payload } => handle_prove(config.as_ref(), payload),
    };

    send_json(&mut stream, &response)?;
    Ok(must_shutdown)
}

fn send_json(stream: &mut TcpStream, response: &Response) -> std::io::Result<()> {
    let json = serde_json::to_string(response)?;
    stream.write_all(json.as_bytes())?;
    stream.flush()
}

// Command handlers
fn handle_status(config: &ServerConfig) -> Response {
    let uptime = config.launch_time.elapsed();
    let status = serde_json::json!({
        "server_id": config.server_id.to_string(),
        "elf_file": config.elf.display().to_string(),
        "uptime": format!("{:.2?}", uptime)
    });
    Response::Ok { message: status.to_string() }
}

fn handle_shutdown() -> Response {
    let msg = serde_json::json!({
        "info": "Shutting down server"
    });
    Response::Ok { message: msg.to_string() }
}
