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

use crate::commands::DEFAULT_PORT;

use colored::Colorize;

#[derive(Parser)]
#[command(name = "Zisk Prover Client", version, about = "Send commands to the prover server")]
pub struct ZiskProveClient {
    #[command(subcommand)]
    pub command: ClientCommand,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "snake_case")]
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

        #[clap(short = 'r', long, default_value_t = false)]
        rma: bool,

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

        /// Use minimal memory
        #[clap(long, default_value_t = false)]
        minimal_memory: bool,

        /// Port of the server (by default DEFAULT_PORT)
        #[clap(long)]
        port: Option<u16>,

        /// Verbosity (-v, -vv)
        #[arg(short ='v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
        verbose: u8, // Using u8 to hold the number of `-v`
    },
    /// Verify constraints from input file
    VerifyConstraints {
        /// Path to the input file
        #[arg(short, long)]
        input: PathBuf,

        /// Port of the server (by default DEFAULT_PORT)
        #[clap(long)]
        port: Option<u16>,

        /// Verbosity (-v, -vv)
        #[arg(short ='v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
        verbose: u8, // Using u8 to hold the number of `-v`
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
                rma,
                final_snark,
                verify_proofs,
                minimal_memory,
                output_dir,
                prefix,
                verbose: _,
                port: _,
            } => ZiskRequest::Prove {
                payload: ZiskProveRequest {
                    input: input.clone(),
                    aggregation: *aggregation,
                    rma: *rma,
                    final_snark: *final_snark,
                    verify_proofs: *verify_proofs,
                    minimal_memory: *minimal_memory,
                    folder: output_dir.clone(),
                    prefix: prefix.clone(),
                },
            },
            ClientCommand::VerifyConstraints { input, verbose: _, port: _ } => {
                ZiskRequest::VerifyConstraints {
                    payload: ZiskVerifyConstraintsRequest { input: input.clone() },
                }
            }
        };

        // Determine the port to use for this client instance.
        // - If no port is specified, default to DEFAULT_PORT.
        // - If a port is specified, use it as the base port.
        // In both cases, the local MPI rank is added to the port to avoid conflicts
        // when running multiple processes on the same machine.
        let port = match self.command {
            ClientCommand::Prove { port, .. }
            | ClientCommand::VerifyConstraints { port, .. }
            | ClientCommand::Status { port }
            | ClientCommand::Shutdown { port } => port.unwrap_or(DEFAULT_PORT),
        };

        // TODO: FIX!
        // port += mpi_context.node_rank as u16;

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
