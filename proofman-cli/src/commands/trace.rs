use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
pub struct Trace {
    #[command(subcommand)]
    pub trace_commands: TraceCommands,
}

#[derive(Subcommand)]
pub enum TraceCommands {
    Setup(TraceSetupCmd),
}

#[derive(Args)]
pub struct TraceSetupCmd {}

impl TraceSetupCmd {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Running setup command");

        Ok(())
    }
}
