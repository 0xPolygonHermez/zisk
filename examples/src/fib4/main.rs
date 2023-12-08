use log::debug;
use std::time::Instant;

use std::path::Path;
use std::sync::Arc;

use proofman::proof_ctx::{ProofCtx, AirInstance};
use proofman::executor::Executor;

use proofman::proofman::{ProofManSettings, ProofMan};
use structopt::StructOpt;

use estark::estark_prover::{ESTARKProver, ESTARKProverSettings};

mod fib4_executor;
use crate::fib4_executor::FibonacciExecutor;

use proto::get_pilout;

fn main() {
    // Setup logging
    env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .filter_level(log::LevelFilter::Debug)
        .init();

    // read command-line args
    let opt = ProofManSettings::from_args();

    match opt {
        ProofManSettings::Prove { debug, airout, output, prover_settings } => {
            if output.exists() { panic!("Output file already exists"); }
            if !airout.exists() { panic!("Airout file does not exist"); }
            if !prover_settings.exists() { panic!("Prover settings file does not exist"); }
        
            prove(debug, airout.as_path(), output.as_path(), prover_settings.as_path());
        },
        ProofManSettings::Verify { .. } => {
            // Add your logic for the 'verify' subcommand here
        },
    }
}

fn prove(_debug: bool, pilout: &Path, _output: &Path, prover_settings: &Path) {
    // read airout file
    let pilout = get_pilout(pilout);

    // Create proof_ctx
    let mut proof_ctx = ProofCtx::new();

    proof_ctx.add_air_instance(AirInstance::new(0, 0));
    proof_ctx.add_air_instance(AirInstance::new(0, 1));
    proof_ctx.add_air_instance(AirInstance::new(1, 0));
    let proof_ctx = Arc::new(proof_ctx);

    // Create executors
    let witness_calculators = vec!(FibonacciExecutor::new());
    
    let cloned_proof_ctx = Arc::clone(&proof_ctx);
    
    witness_calculators[0].witness_computation(1, 0, 0, cloned_proof_ctx);
    
    // Prover
    // read prover settings file
    let estark_settings = std::fs::read_to_string(prover_settings).expect("Error reading settings file");
    let estark_settings = ESTARKProverSettings::new(estark_settings);
    let prover = ESTARKProver::new(estark_settings);

    let proofman = ProofMan::new(&prover/* , witness_calculators, airout*/);
    // proofman.setup(setup);

    let now = Instant::now();
    // const public_inputs = { in1: 1n, in2: 2n };
    // proofman.prove(public_inputs);
    debug!("Proof generated in {} ms", now.elapsed().as_millis());

    // ONLY for demonstration purposes - remove this in production
    // Now you can use the parsed data as needed
    println!("Name: {:?}", pilout.name);
    println!("Base Field Length: {}", pilout.base_field.len());

    // Access nested structures
    for subproof in &pilout.subproofs {
        println!("Subproof Name: {:?}", subproof.name);
        for air in &subproof.airs {
            println!("Air Name: {:?}", air.name);
            println!("Num rows: {:?}", air.num_rows);
            println!("Air Constraint Count: {}", air.constraints.len());
        }
    }
}
