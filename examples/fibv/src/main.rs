use log::debug;

use goldilocks::{Goldilocks, AbstractField};

use proofman::public_inputs::PublicInputs;
use prover_mocked::mocked_prover_builder::MockedProverBuilder;

mod executor_fibo;
use executor_fibo::FibonacciExecutor;

mod executor_module;
use executor_module::ModuleExecutor;

use proofman::proof_manager::ProofManager;

use proofman::proof_manager_config::ProofManConfig;
use proofman::proofman_cli::ProofManCli;

use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Serialize, Deserialize)]
pub struct FibVPublicInputs<T> {
    a: T,
    b: T,
    module: T,
}

impl FibVPublicInputs<u64> {
    pub fn new(json: &str) -> FibVPublicInputs<Goldilocks> {
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
    fn to_vec(&self) -> Vec<Goldilocks> {
        vec![self.a, self.b, self.module]
    }
}

fn main() {
    env_logger::builder().format_timestamp(None).format_target(false).filter_level(log::LevelFilter::Trace).init();

    let arguments = ProofManCli::read_arguments();
    let config_json = std::fs::read_to_string(arguments.config).expect("Failed to read file");
    let proofman_config = ProofManConfig::parse_input_json(&config_json);

    //read public inputs file
    let public_inputs_filename = arguments.public_inputs.as_ref().unwrap().display().to_string();
    let public_inputs = match std::fs::read_to_string(&public_inputs_filename) {
        Ok(public_inputs) => FibVPublicInputs::new(&public_inputs),
        Err(err) => {
            println!("Error reading public inputs file '{}': {}", &public_inputs_filename, err);
            std::process::exit(1);
        }
    };

    let fibonacci_executor = Box::new(FibonacciExecutor::new());
    let module_executor = Box::new(ModuleExecutor::new());

    let prover_builder = MockedProverBuilder::<Goldilocks>::new();

    let mut proofman =
        ProofManager::new(proofman_config, vec![fibonacci_executor, module_executor], Box::new(prover_builder));

    let now = std::time::Instant::now();
    let proof = proofman.prove(Some(Box::new(public_inputs)));
    if let Err(err) = proof {
        println!("Error: {}", err);
    }

    debug!("Proof generated in {} ms", now.elapsed().as_millis());
}
