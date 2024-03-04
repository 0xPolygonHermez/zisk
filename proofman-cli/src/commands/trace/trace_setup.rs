use clap::Args;
use proofman::command_handlers::trace_setup_handler::trace_setup_handler;
use std::path::PathBuf;
use colored::Colorize;

#[derive(Args)]
pub struct TraceSetupCmd {
    /// pilout file path
    #[clap(short, long)]
    pub pilout: PathBuf,

    /// destination folder path
    #[clap(short, long, default_value = ".")]
    pub dest: PathBuf,
}

impl TraceSetupCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} {}", format!("{: >12}", "Command").bright_green().bold(), "Trace setup subcommand");
        println!("");

        trace_setup_handler(&self.pilout, &self.dest)
    }
}
