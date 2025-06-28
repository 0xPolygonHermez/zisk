use anyhow::Result;
use clap::{Parser, Subcommand};
use server::{
    ZiskProveRequest, ZiskRequest, ZiskResponse, ZiskShutdownRequest, ZiskStatusRequest,
    ZiskVerifyConstraintsRequest,
};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::PathBuf,
};

use crate::commands::{initialize_mpi, DEFAULT_PORT};

use colored::Colorize;

#[derive(Parser)]
#[command(name = "Zisk Prover Client", version, about = "Send commands to the prover server")]
pub struct ZiskProveClient {
    #[command(subcommand)]
    pub command: ClientCommand,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "lowercase")]
pub enum ClientCommand {
    /// Get server status
    Status {
        /// Port of the server (by default DEFAULT_PORT)
        #[clap(long)]
        port: Option<u16>,
    },

    /// Shut down the server
    Shutdown {
        /// Port of the server (by default DEFAULT_PORT)
        #[clap(long)]
        port: Option<u16>,
    },

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

        /// Port of the server (by default DEFAULT_PORT)
        #[clap(long)]
        port: Option<u16>,
    },
    /// Verify constraints from input file
    VerifyConstraints {
        /// Path to the input file
        #[arg(short, long)]
        input: PathBuf,

        /// Port of the server (by default DEFAULT_PORT)
        #[clap(long)]
        port: Option<u16>,
    },
}

impl ZiskProveClient {
    pub fn run(&self) -> Result<()> {
        let request = match &self.command {
            ClientCommand::Status { port: _ } => {
                ZiskRequest::Status { payload: ZiskStatusRequest {} }
            }
            ClientCommand::Shutdown { port: _ } => {
                ZiskRequest::Shutdown { payload: ZiskShutdownRequest {} }
            }
            ClientCommand::Prove {
                input,
                aggregation,
                final_snark,
                verify_proofs,
                output_dir,
                prefix,
                port: _,
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
            ClientCommand::VerifyConstraints { input, port: _ } => ZiskRequest::VerifyConstraints {
                payload: ZiskVerifyConstraintsRequest { input: input.clone() },
            },
        };

        // Construct server address
        let mpi_context = initialize_mpi()?;

        proofman_common::initialize_logger(
            proofman_common::VerboseMode::Info,
            Some(mpi_context.world_rank),
        );

        // Determine the port to use for this client instance.
        // - If no port is specified, default to DEFAULT_PORT.
        // - If a port is specified, use it as the base port.
        // In both cases, the local MPI rank is added to the port to avoid conflicts
        // when running multiple processes on the same machine.
        let mut port = match self.command {
            ClientCommand::Prove { port, .. }
            | ClientCommand::VerifyConstraints { port, .. }
            | ClientCommand::Status { port }
            | ClientCommand::Shutdown { port } => port.unwrap_or(DEFAULT_PORT),
        };
        port += mpi_context.local_rank as u16;

        let address = format!("localhost:{port}");

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

        if let Err(e) = serde_json::from_str::<ZiskResponse>(&response_line) {
            return Err(anyhow::anyhow!(
                "Failed to parse server response: {}\nRaw: {}",
                e,
                response_line
            ));
        }

        println!();
        println!("{} {}", format!("{: >12}", "Response").bright_green().bold(), response_line);

        Ok(())
    }
}
