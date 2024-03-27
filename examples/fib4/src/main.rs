use log::debug;
use goldilocks::Goldilocks;

use prover_mocked::mocked_prover_builder::MockedProverBuilder;

mod executor;
use crate::executor::FibonacciExecutor;

use clap::Parser;
use proofman::proof_manager::ProofManager;
use proofman::proof_manager_config::ProofManConfig;
use proofman_cli::commands::prove::ProveCmd;

fn main() {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    let arguments: ProveCmd = ProveCmd::parse();
    let config_json = std::fs::read_to_string(arguments.config).expect("Failed to read file");

    let proofman_config = ProofManConfig::parse_input_json(&config_json);

    let executor = Box::new(FibonacciExecutor::new());

    let prover_builder = MockedProverBuilder::<Goldilocks>::new();

    let mut proofman = match ProofManager::<Goldilocks>::new(proofman_config, vec![executor], Box::new(prover_builder))
    {
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
