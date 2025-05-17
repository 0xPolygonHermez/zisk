use anyhow::Result;
use clap::{Parser, Subcommand};
use server::{ProveRequest, Request, Response, VerifyConstraintsRequest};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::PathBuf,
};

#[derive(Parser)]
#[command(name = "Zisk Prover Client", version, about = "Send commands to the prover server")]
pub struct ZiskProveClient {
    /// Address of the server (e.g., 127.0.0.1:7878)
    pub address: String,

    #[command(subcommand)]
    pub command: ClientCommand,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "lowercase")]
pub enum ClientCommand {
    /// Get server status
    Status,

    /// Shut down the server
    Shutdown,

    Prove {
        /// Path to the input file
        #[arg(short, long)]
        input: PathBuf,

        /// Use aggregation
        #[clap(short = 'a', long, default_value_t = false)]
        aggregation: bool,

        /// Use final snark
        #[clap(short = 'f', long, default_value_t = false)]
        final_snark: bool,

        /// Verify proofs
        #[clap(short = 'y', long, default_value_t = false)]
        verify_proofs: bool,
    },
    /// Verify constraints from input file
    VerifyConstraints {
        /// Path to the input file
        #[arg(short, long)]
        input: PathBuf,
    },
}

impl ZiskProveClient {
    pub fn run(&self) -> Result<()> {
        let request = match &self.command {
            ClientCommand::Status => Request::Status,
            ClientCommand::Shutdown => Request::Shutdown,
            ClientCommand::Prove { input, aggregation, final_snark, verify_proofs } => {
                Request::Prove {
                    payload: ProveRequest {
                        input: input.clone(),
                        aggregation: *aggregation,
                        final_snark: *final_snark,
                        verify_proofs: *verify_proofs,
                    },
                }
            }
            ClientCommand::VerifyConstraints { input } => Request::VerifyConstraints {
                payload: VerifyConstraintsRequest { input: input.clone() },
            },
        };

        // Open connection
        let mut stream = TcpStream::connect(&self.address)
            .map_err(|e| anyhow::anyhow!("Failed to connect to server: {}", e))?;

        // Serialize and send request
        let mut request_json = serde_json::to_string(&request)?;
        request_json.push('\n');
        stream.write_all(request_json.as_bytes())?;

        // Read and parse response
        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;

        let response: Response = serde_json::from_str(&response_line)
            .unwrap_or(Response::Error { message: "Failed to parse response".to_string() });

        // Handle response
        match response {
            Response::Ok { message } => println!("Success: {}", message),
            Response::Error { message } => eprintln!("Error: {}", message),
        }

        Ok(())
    }
}
