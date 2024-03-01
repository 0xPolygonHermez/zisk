// extern crate env_logger;
use clap::Parser;

use util::cli::{GREEN, RESET};

// #[derive(Parser)]
// #[command(version, about, long_about = None)]
// #[command(propagate_version = true)]
// pub struct Cli {
//     #[command(subcommand)]
// }
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ProveCmd {
    /// Proofman configuration file path
    #[clap(short, long, default_value = "proofman.config.json")]
    pub config: String,

    /// Output file path
    #[clap(short, long, default_value = "proof.json")]
    pub output: String,

    /// Public inputs file path
    #[clap(short, long)]
    pub public_inputs: Option<String>,
}

impl ProveCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}{}{} {}", GREEN, format!("{: >12}", "Command"), RESET, "Prove command");
        println!("");

        Ok(())
    }
}
