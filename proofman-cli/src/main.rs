use clap::{Parser, Subcommand};
mod commands;
use commands::trace::{TraceCommands, Trace};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Trace(Trace),
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Trace(args) => match &args.trace_commands {
            TraceCommands::Setup(args) => {
                args.run().unwrap();
            }
        },
    };
}
