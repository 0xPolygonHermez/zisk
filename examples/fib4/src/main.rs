use log::debug;

use goldilocks::Goldilocks;
use prover_mocked::mocked_prover_builder::MockedProverBuilder;

use std::time::Instant;

mod executor;
use crate::executor::FibonacciExecutor;

use proofman::proof_manager::ProofManager;

use proofman::proof_manager_config::ProofManConfig;
use proofman::proofman_cli::ProofManCli;

fn main() {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    let arguments = ProofManCli::read_arguments();
    let config_json = std::fs::read_to_string(arguments.config).expect("Failed to read file");

    let proofman_config = ProofManConfig::parse_input_json(&config_json);

    let executor = Box::new(FibonacciExecutor::new(None));

    let prover_builder = MockedProverBuilder::<Goldilocks>::new();

    let mut proofman = ProofManager::<Goldilocks>::new(proofman_config, vec![executor], Box::new(prover_builder));

    let now = Instant::now();
    proofman.prove(None);
    debug!("Proof generated in {} ms", now.elapsed().as_millis());
}
