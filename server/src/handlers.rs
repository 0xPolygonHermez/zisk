use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    sync::Arc,
};

use crate::ServerConfig;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Serialize, Deserialize, Debug, Clone, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[clap(rename_all = "lowercase")]
pub enum Command {
    Summary,
    Shutdown,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub command: Command,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "status")]
#[serde(rename_all = "lowercase")]
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

    info!("Received request: {:?}", request);

    let must_shutdown = request.command == Command::Shutdown;
    let response = match request.command {
        Command::Summary => handle_summary(config.as_ref()),
        Command::Shutdown => handle_shutdown(),
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
fn handle_summary(config: &ServerConfig) -> Response {
    let uptime = config.launch_time.elapsed();
    let summary = serde_json::json!({
        "server_id": config.server_id.to_string(),
        "elf_file": config.elf.display().to_string(),
        "uptime": format!("{:.2?}", uptime)
    });
    Response::Ok { message: summary.to_string() }
}

fn handle_shutdown() -> Response {
    let msg = serde_json::json!({
        "info": "Shutting down server"
    });
    Response::Ok { message: msg.to_string() }
}
