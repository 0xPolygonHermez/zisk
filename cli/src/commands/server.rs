use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use server::{Server, ServerConfig};
use std::fs::OpenOptions;
use std::{path::PathBuf, process};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::ux::print_banner;

const DEFAULT_PORT: u16 = 7878;

// Structure representing the 'prove' subcommand of cargo.
#[derive(Parser, Debug)]
#[command(name = "Prover Server", version, about = "A TCP-based prover control server", long_about = None)]
pub struct ZiskServer {
    /// Path to the ELF file
    elf_file: PathBuf,

    /// Optional port number (default 7878)
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,
}

impl ZiskServer {
    pub fn run(&self) -> Result<()> {
        init_tracing("zisk_prover_server.log");

        if !self.elf_file.exists() {
            eprintln!("Error: ELF file '{}' not found.", self.elf_file.display());
            process::exit(1);
        }

        print_banner();
        println!("{} Proving Server", format!("{: >12}", "Command").bright_green().bold());
        println!();

        let config = ServerConfig::new(self.elf_file.clone(), self.port);
        if let Err(e) = Server::new(config).run() {
            eprintln!("Error starting server: {}", e);
            process::exit(1);
        }

        Ok(())
    }
}

fn init_tracing(log_path: &str) {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_path)
        .expect("Failed to open log file");

    let file_layer = fmt::layer()
        .with_writer(file)
        .with_ansi(false) // no color in file
        .with_target(false);

    let stdout_layer = fmt::layer().with_writer(std::io::stdout).with_ansi(true).with_target(false);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with(stdout_layer)
        .with(file_layer)
        .init();
}
