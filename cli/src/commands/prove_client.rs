use anyhow::Result;
use clap::{Parser, Subcommand};
use server::{ZiskProveRequest, ZiskRequest, ZiskResponse, ZiskVerifyConstraintsRequest};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::PathBuf,
};

use crate::commands::initialize_mpi;

#[derive(Parser)]
#[command(name = "Zisk Prover Client", version, about = "Send commands to the prover server")]
pub struct ZiskProveClient {
    /// Port of the server (e.g., 7878)
    pub port: u16,

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

        /// Output folder for the proof
        #[clap(short = 'o', long, default_value = "tmp")]
        output_dir: PathBuf,

        #[clap(short = 'p')]
        prefix: String,
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
            ClientCommand::Status => ZiskRequest::Status,
            ClientCommand::Shutdown => ZiskRequest::Shutdown,
            ClientCommand::Prove {
                input,
                aggregation,
                final_snark,
                verify_proofs,
                output_dir,
                prefix,
            } => ZiskRequest::Prove {
                payload: ZiskProveRequest {
                    input: input.clone(),
                    aggregation: *aggregation,
                    final_snark: *final_snark,
                    verify_proofs: *verify_proofs,
                    folder: output_dir.clone(),
                    prefix: prefix.clone(),
                },
            },
            ClientCommand::VerifyConstraints { input } => ZiskRequest::VerifyConstraints {
                payload: ZiskVerifyConstraintsRequest { input: input.clone() },
            },
        };

        // Construct server address
        let mpi_context = initialize_mpi()?;
        let port = self.port + mpi_context.local_rank as u16;

        let address = format!("localhost:{}", port);

        // Open connection
        let mut stream = TcpStream::connect(&address)
            .map_err(|e| anyhow::anyhow!("Failed to connect to server: {}", e))?;

        // Serialize and send request
        let mut request_json = serde_json::to_string(&request)?;
        request_json.push('\n');
        stream.write_all(request_json.as_bytes())?;

        // Read and parse response
        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;

        let response: ZiskResponse = serde_json::from_str(&response_line)
            .unwrap_or(ZiskResponse::Error { message: "Failed to parse response".to_string() });

        // Handle response
        match response {
            ZiskResponse::Ok { message } => println!("Success: {}", message),
            ZiskResponse::Error { message } => eprintln!("Error: {}", message),
        }

        Ok(())
    }
}
