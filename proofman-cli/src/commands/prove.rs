// extern crate env_logger;
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ProveCmd {
    /// Proofman configuration file path
    #[clap(short, long, default_value = "proofman.config.json")]
    pub config: PathBuf,

    /// Output file path
    #[clap(short, long, default_value = "proof.json")]
    pub output: PathBuf,

    /// Public inputs file path
    #[clap(short, long)]
    pub public_inputs: Option<PathBuf>,
}

impl ProveCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} {}", format!("{: >12}", "Command").bright_green().bold(), "Prove command");
        println!("");

        Ok(())
    }
}
