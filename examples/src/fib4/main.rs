use log::debug;
use math::fields::f64::BaseElement;
use std::time::Instant;

use estark::estark_prover::{ESTARKProver, ESTARKProverSettings};

mod executor;
use crate::executor::FibonacciExecutor;

use std::path::PathBuf;
use structopt::StructOpt;

use proofman::proof_manager::{ProofManager, ProofManOpt};

#[derive(StructOpt)]
#[structopt(name = "fib4", about = "Fibonacci 4 proofman example")]
struct Fib4Options {
    /// De/Activate debug mode
    #[structopt(short, long)]
    _debug: bool,

    // TODO: Public inputs as Option
    
    /// Prover settings file
    #[structopt(short, long, parse(from_os_str))]
    prover_settings: PathBuf,

    /// Output file
    #[structopt(short, long, parse(from_os_str))]
    output: PathBuf,
}

fn main() {
    // read command-line args
    let opt = Fib4Options::from_args();

    // CHECKS 
    // Check if prover settings file exists
    if !opt.prover_settings.exists() {
        eprintln!("Error: Prover settings file '{}' does not exist", opt.prover_settings.display());
        std::process::exit(1);
    }

    // Check if output file already exists
    if opt.output.exists() {
        eprintln!("Error: Output file '{}' already exists", opt.output.display());
        std::process::exit(1);
    }

    // Create prover
    // read prover settings file
    let estark_settings = match std::fs::read_to_string(&opt.prover_settings) {
        Ok(settings) => ESTARKProverSettings::new(settings),
        Err(err) => {
            eprintln!("Error reading settings file '{}': {}", opt.prover_settings.display(), err);
            std::process::exit(1);
        }
    };

    let options = ProofManOpt {
        debug: opt._debug,
        ..ProofManOpt::default()
    };

    type GoldyLocks = BaseElement;
    let prover = ESTARKProver::new(estark_settings, /* prover_options */);
    let executor = Box::new(FibonacciExecutor::new("Fibonacci".to_string()));
    let mut proofman = ProofManager::<GoldyLocks>::new(
        "examples/src/fib4/fib4.pilout",
        vec!(executor),
        Box::new(prover),
        options
    );

    let now = Instant::now();
    proofman.prove(None);
    debug!("Proof generated in {} ms", now.elapsed().as_millis());
}