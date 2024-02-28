use clap::{Parser, Subcommand};
mod commands;
use commands::trace_commands::{TraceSubcommands, Trace};

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
            TraceSubcommands::Setup(args) => {
                args.run().unwrap();
            }
        },
    };
}
