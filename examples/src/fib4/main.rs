use std::time::Instant;
use log::debug;

use std::path::Path;
use proofman::proof_ctx::{ProofCtx, AirInstance};
use proofman::executor::Executor;
use std::sync::Arc;

mod fib4_executor;

// Move this entire block to proofman_options.rs inside proofman when it is ready
use std::path::PathBuf;
use structopt::StructOpt;

use estark::estark_prover::ESTARKProverSettings;

use crate::fib4_executor::FibonacciExecutor;

use proto::get_pilout;

#[derive(StructOpt, Debug)]
#[structopt(name = "proofman", about = "Proofman CLI")]
pub enum ProofManOptions {
    /// Prove
    #[structopt(name = "prove")]
    Prove {
        /// De/Activate debug mode
        #[structopt(short, long)]
        debug: bool,

        // TODO: Public inputs as Option

        /// Airout file
        #[structopt(short, long, parse(from_os_str))]
        airout: PathBuf, 
        
        /// Prover settings file
        #[structopt(short, long, parse(from_os_str))]
        prover_settings: PathBuf,

        /// Output file
        #[structopt(short, long, parse(from_os_str))]
        output: PathBuf,
    },
    Verify {

    }
}
// End of block


// EXAMPLE FIBONACCI
// ================================================================================================

fn main() {
    // Setup logging
    env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .filter_level(log::LevelFilter::Debug)
        .init();

    // read command-line args
    let opt = ProofManOptions::from_args();

    match opt {
        ProofManOptions::Prove { debug, airout, output, prover_settings } => {
            if output.exists() { panic!("Output file already exists"); }
            if !airout.exists() { panic!("Airout file does not exist"); }
            if !prover_settings.exists() { panic!("Prover settings file does not exist"); }
        
            prove(debug, airout.as_path(), output.as_path(), prover_settings.as_path());
        },
        ProofManOptions::Verify { .. } => {
            // Add your logic for the 'verify' subcommand here
        },
    }
}

fn prove(_debug: bool, pilout: &Path, _output: &Path, prover_settings: &Path) {
    // read airout file
    let pilout = get_pilout(pilout);

    // read prover settings file
    let estark_settings = std::fs::read_to_string(prover_settings).expect("Error reading settings file");
    let _estark_settings = ESTARKProverSettings::new(estark_settings);

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
    // let prover = eSTark::new(proverOptions)

    // const public_inputs = { in1: 1n, in2: 2n };

    // let proofman = ProofMan::new(prover, witness_calculators, airout);
    // proofman.setup(setup);

    let now = Instant::now();
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
