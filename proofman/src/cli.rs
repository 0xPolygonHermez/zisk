use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "proofman", about = "Proofman")]
pub struct ProofmanCli {
    /// De/Activate debug mode
    #[structopt(short, long)]
    pub _debug: bool,

    /// Prover settings file
    #[structopt(short, long, parse(from_os_str))]
    pub proofman_settings: PathBuf,

    /// Output file
    #[structopt(short, long, parse(from_os_str))]
    pub output: PathBuf,
}

impl ProofmanCli {
    pub fn read_arguments() -> ProofmanCli {
        // read command-line args
        let arg = ProofmanCli::from_args();

        // CHECKS
        // Check if prover settings file exists
        if !arg.proofman_settings.exists() {
            eprintln!("Error: Prover settings file '{}' does not exist", arg.proofman_settings.display());
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
