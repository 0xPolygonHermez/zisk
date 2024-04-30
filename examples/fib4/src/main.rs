use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::null_mut;

use log::debug;
use goldilocks::Goldilocks;

use proofman::executor::Executor;
use prover_mocked::mocked_prover_builder::MockedProverBuilder;
use zkevm_lib_c::ffi::zkin_new_c;

mod executor;
use crate::executor::FibonacciExecutor;

use clap::Parser;
use proofman::proof_manager::ProofManager;
use proofman::proof_manager_config::ProofManConfig;
use proofman_cli::commands::prove::ProveCmd;

fn main() {
    let void_1: *mut c_void = null_mut();
    let void_2: *mut c_void = null_mut();
    let vec_goldi_1 = Vec::<Goldilocks>::new();
    let vec_goldi_2 = Vec::<Goldilocks>::new();

    zkin_new_c(void_1, void_2, &vec_goldi_1, &vec_goldi_2);

    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    let arguments: ProveCmd = ProveCmd::parse();
    let config_json = std::fs::read_to_string(arguments.config).expect("Failed to read file");

    let proofman_config = ProofManConfig::parse_input_json(&config_json);

    let fibonacci_executor = FibonacciExecutor::new();
    let executor_vec: Vec<&dyn Executor<Goldilocks>> = vec![&fibonacci_executor];

    let prover_builder = MockedProverBuilder::<Goldilocks>::new();

    let mut prover_builders = HashMap::new();
    prover_builders.insert("Fibonacci".to_string(), prover_builder);

    let mut proofman = match ProofManager::new(proofman_config, executor_vec, prover_builders, None, false) {
        Ok(proofman) => proofman,
        Err(err) => {
            println!("Error: {:?}", err);
            return;
        }
    };

    let now = std::time::Instant::now();
    let proof = proofman.prove(None);
    if let Err(err) = proof {
        println!("Error: {}", err);
    }
    debug!("Proof generated in {} ms", now.elapsed().as_millis());
}
