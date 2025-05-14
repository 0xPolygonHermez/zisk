use anyhow::Result;
use clap::Parser;
use server::{Command, Request, Response};
use std::{
    io::{Read, Write},
    net::TcpStream,
};

// Structure representing the 'prove' subcommand of cargo.
#[derive(Parser)]
#[command(name = "Zisk Prover Client", version, about = "Send commands to the prover server")]
pub struct ZiskProveClient {
    /// Address of the server (e.g., 127.0.0.1:7878)
    address: String,

    /// Command to send
    #[arg(value_enum)]
    pub command: Command,
}

impl ZiskProveClient {
    pub fn run(&self) -> Result<()> {
        let request = Request { command: self.command.clone() };

        let mut stream = match TcpStream::connect(&self.address) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to connect to server: {}", e);
                std::process::exit(1);
            }
        };

        let mut request_json = serde_json::to_string(&request).unwrap();
        request_json.push('\n');

        if let Err(e) = stream.write_all(request_json.as_bytes()) {
            eprintln!("Failed to send request: {}", e);
            std::process::exit(1);
        }

        let mut buffer = String::new();
        if let Err(e) = stream.read_to_string(&mut buffer) {
            eprintln!("Failed to read response: {}", e);
            std::process::exit(1);
        }

        let response: Response = serde_json::from_str(&buffer)
            .unwrap_or(Response::Error { message: "Failed to parse response".to_string() });

        match response {
            Response::Ok { message } => println!("Success: {}", message),
            Response::Error { message } => eprintln!("Error: {}", message),
        }

        Ok(())
    }
}
