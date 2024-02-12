use log::debug;

use goldilocks::Goldilocks;

use std::time::Instant;

use prover_mocked::mocked_prover::MockedProver;

mod executor;
use crate::executor::FibonacciExecutor;

use proofman::proof_manager::{ProofManager, ProofManSettings};
use proofman::proof_manager_config::ConfigNull;

use estark::config::{executors_config::ExecutorsConfig, prover_config::EStarkConfig, meta_config::MetaConfig};
use proofman::proof_manager_config::ProofManConfig;
use proofman::cli::ProofmanCli;

fn main() {
    env_logger::builder().format_timestamp(None).format_target(false).filter_level(log::LevelFilter::Trace).init();

    let arguments = ProofmanCli::read_arguments();

    let config_json = std::fs::read_to_string(arguments.proofman_settings).expect("Failed to read file");

    let proofman_config = ProofManConfig::<ExecutorsConfig, EStarkConfig, MetaConfig>::parse_input_json(&config_json);

    let executor = Box::new(FibonacciExecutor::new());

    let prover = MockedProver::<Goldilocks>::new();

    let mut proofman = ProofManager::<Goldilocks>::new(
        proofman_config.get_pilout(),
        vec![executor],
        Box::new(prover),
        Box::new(ConfigNull {}),
        ProofManSettings { debug: arguments._debug, ..ProofManSettings::default() },
    );

    let now = Instant::now();
    proofman.prove(None);
    debug!("Proof generated in {} ms", now.elapsed().as_millis());
}
