use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "proofman", about = "Proofman")]
pub struct ProofManCli {
    /// Prover settings file
    #[structopt(short, long, parse(from_os_str))]
    pub config: PathBuf,

    /// Output file
    #[structopt(short, long, parse(from_os_str))]
    pub output: PathBuf,
}

impl ProofManCli {
    pub fn read_arguments() -> ProofManCli {
        // read command-line args
        let arg = ProofManCli::from_args();

        // CHECKS
        // Check if prover settings file exists
        if !arg.config.exists() {
            eprintln!("Error: Prover settings file '{}' does not exist", arg.config.display());
            std::process::exit(1);
        }

        // Check if output file already exists
        if arg.output.exists() {
            eprintln!("Error: Output file '{}' already exists", arg.output.display());
            std::process::exit(1);
        }

        arg
    }
}
