use log::debug;

use goldilocks::Goldilocks;

use std::time::Instant;

use prover_mocked::mocked_prover::MockedProver;

mod executor;
use crate::executor::FibonacciExecutor;

use std::path::PathBuf;
use structopt::StructOpt;

use proofman::proof_manager::{ProofManager, ProofManSettings};
use proofman::config::ConfigNull;
use estark::config::executors_config::ExecutorsConfig;
use estark::config::prover_config::EStarkConfig;
use estark::config::meta_config::MetaConfig;
use proofman::config::proofman_config::Config;

#[derive(StructOpt)]
#[structopt(name = "fib4", about = "Fibonacci 4 proofman example")]
struct Fib4Options {
    /// De/Activate debug mode
    #[structopt(short, long)]
    _debug: bool,

    /// Prover settings file
    #[structopt(short, long, parse(from_os_str))]
    proofman_settings: PathBuf,

    /// Output file
    #[structopt(short, long, parse(from_os_str))]
    output: PathBuf,
}

fn read_arguments() -> Fib4Options {
    // read command-line args
    let opt = Fib4Options::from_args();

    // CHECKS
    // Check if prover settings file exists
    if !opt.proofman_settings.exists() {
        eprintln!("Error: Prover settings file '{}' does not exist", opt.proofman_settings.display());
        std::process::exit(1);
    }

    // Check if output file already exists
    if opt.output.exists() {
        eprintln!("Error: Output file '{}' already exists", opt.output.display());
        std::process::exit(1);
    }

    opt
}

fn main() {
    env_logger::builder().format_timestamp(None).format_target(false).filter_level(log::LevelFilter::Trace).init();

    let arguments = read_arguments();

    let config_json = std::fs::read_to_string(arguments.proofman_settings).expect("Failed to read file");

    let config = Config::<ExecutorsConfig, EStarkConfig, MetaConfig>::parse_input_json(&config_json);

    let executor = Box::new(FibonacciExecutor::new());

    let prover = MockedProver::<Goldilocks>::new();

    let mut proofman = ProofManager::<Goldilocks>::new(
        config.get_pilout(),
        vec![executor],
        Box::new(prover),
        Box::new(ConfigNull {}),
        ProofManSettings { debug: arguments._debug, ..ProofManSettings::default() },
    );

    let now = Instant::now();
    proofman.prove(None);
    debug!("Proof generated in {} ms", now.elapsed().as_millis());
}
