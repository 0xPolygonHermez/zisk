use std::process::exit;

use clap::{Parser, Subcommand};
mod commands;
use commands::pil_helpers::PilHelpersCmd;
// use commands::new::NewCmd;
use commands::prove::ProveCmd;
use commands::verify_constraints::VerifyConstraintsCmd;
// use commands::trace::{TraceSubcommands, TraceCmd};
use commands::pilout::{PiloutSubcommands, PiloutCmd};
use proofman_util::cli::print_banner;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Pilout(PiloutCmd),
    // Trace(TraceCmd),
    // New(NewCmd),
    Prove(ProveCmd),
    PilHelpers(PilHelpersCmd),
    VerifyConstraints(VerifyConstraintsCmd)
}

fn main() {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    print_banner(false);

    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Pilout(args) => match &args.pilout_commands {
            PiloutSubcommands::Inspect(args) => args.run(),
        },
        // Commands::Trace(args) => match &args.trace_commands {
        //     TraceSubcommands::Setup(args) => {
        //         args.run().unwrap();
        //     }
        // },
        // Commands::New(args) => {
        //     args.run().unwrap();
        // }
        Commands::Prove(args) => args.run(),
        Commands::PilHelpers(args) => args.run(),
        Commands::VerifyConstraints(args) => args.run(),
    };

    if let Err(e) = result {
        log::error!("{}", e);
        exit(1);
    }

    log::info!("Done");
}
