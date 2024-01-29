use log::debug;
use goldilocks::{Goldilocks, AbstractField};
use std::time::Instant;
use proofman::public_inputs::PublicInputs;

use estark::estark_prover::{ESTARKProver, ESTARKProverSettings};

mod executor_fibo;
use executor_fibo::FibonacciExecutor;

mod executor_module;
use executor_module::ModuleExecutor;

use serde::{Deserialize, Serialize};
use serde_json;

use std::path::PathBuf;
use structopt::StructOpt;

use proofman::proof_manager::{ProofManager, ProofManOpt};

#[derive(StructOpt)]
#[structopt(name = "fibv", about = "Fibonacci 4 proofman example")]
struct FibVOptions {
    /// De/Activate debug mode
    #[structopt(short, long)]
    _debug: bool,

    /// Public inputs file
    #[structopt(long, parse(from_os_str))]
    public_inputs: PathBuf,

    /// Prover settings file
    #[structopt(short, long, parse(from_os_str))]
    prover_settings: PathBuf,

    /// Output file
    #[structopt(short, long, parse(from_os_str))]
    output: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FibVPublicInputs<T> {
    a: T,
    b: T,
    module: T,
}

impl FibVPublicInputs<u64> {
    pub fn new(json: String) -> FibVPublicInputs<Goldilocks> {
        let data: Result<FibVPublicInputs<u64>, _> = serde_json::from_str(&json);

        match data {
            Ok(data) => FibVPublicInputs {
                a: Goldilocks::from_canonical_u64(data.a),
                b: Goldilocks::from_canonical_u64(data.b),
                module: Goldilocks::from_canonical_u64(data.module),
            },
            Err(e) => panic!("Error parsing settings file: {}", e),
        }
    }
}

impl<Goldilocks: Copy + Send + Sync + std::fmt::Debug> PublicInputs<Goldilocks> for FibVPublicInputs<Goldilocks> {
    fn to_elements(&self) -> Vec<Goldilocks> {
        vec![self.a, self.b, self.module]
    }
}

fn main() {
    env_logger::builder().format_timestamp(None).format_target(false).filter_level(log::LevelFilter::Trace).init();

    // read command-line args
    let opt = FibVOptions::from_args();

    // CHECKS
    // Check if public inputs file exists
    if !opt.public_inputs.exists() {
        eprintln!("Error: Public inputs file '{}' does not exist", opt.public_inputs.display());
        std::process::exit(1);
    }

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

    //read public inputs file
    let public_inputs = match std::fs::read_to_string(&opt.public_inputs) {
        Ok(public_inputs) => FibVPublicInputs::new(public_inputs),
        Err(err) => {
            eprintln!("Error reading public inputs file '{}': {}", opt.public_inputs.display(), err);
            std::process::exit(1);
        }
    };

    let options = ProofManOpt { debug: opt._debug, ..ProofManOpt::default() };

    let prover = ESTARKProver::new(estark_settings /* prover_options */);

    let executor = Box::new(FibonacciExecutor::new());
    let module1 = Box::new(ModuleExecutor::new());
    let module2 = Box::new(ModuleExecutor::new());

    let mut proofman = ProofManager::<Goldilocks>::new(
        "examples/fibv/src/fibv.pilout",
        vec![module2, executor, module1],
        Box::new(prover),
        "".to_owned(),
        options,
    );

    let now = Instant::now();
    proofman.prove(Some(Box::new(public_inputs)));
    debug!("Proof generated in {} ms", now.elapsed().as_millis());
}
