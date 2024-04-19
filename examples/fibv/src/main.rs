use log::debug;

use goldilocks::{Goldilocks, AbstractField};

use prover_mocked::mocked_prover_builder::MockedProverBuilder;

mod executor_fibo;
use executor_fibo::FibonacciExecutor;

mod executor_module;
use executor_module::ModuleExecutor;

use proofman::{executor::Executor, proof_manager::ProofManager};

use proofman::proof_manager_config::ProofManConfig;

use serde::{Deserialize, Serialize};
use serde_json;

use clap::Parser;
use proofman_cli::commands::prove::ProveCmd;
use stark::stark_prover_settings::StarkProverSettings;

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

impl<T> Into<Vec<T>> for FibVPublicInputs<T> {
    fn into(self) -> Vec<T> {
        vec![self.a, self.b, self.module]
    }
}

fn main() {
    env_logger::builder().format_timestamp(None).format_target(false).filter_level(log::LevelFilter::Trace).init();

    let arguments: ProveCmd = ProveCmd::parse();

    let config_json = std::fs::read_to_string(arguments.config).expect("Failed to read file");
    let proofman_config = ProofManConfig::parse_input_json(&config_json);

    // let stark_config = StarkProverSettings {
    //     current_path: "",
    //     const_pols_filename: "",
    //     map_const_pols_file: "",
    //     const_tree_filename: "",
    //     stark_info_filename: "",
    //     verkey_filename: "",
    //     chelpers_filename: "",        
    // }






    //read public inputs file
    let public_inputs_filename = arguments.public_inputs.as_ref().unwrap();
    let public_inputs = match std::fs::read_to_string(&public_inputs_filename) {
        Ok(public_inputs) => FibVPublicInputs::new(&public_inputs),
        Err(err) => {
            println!("Error reading public inputs file '{}': {}", &public_inputs_filename.display(), err);
            std::process::exit(1);
        }
    };

    let fibonacci_executor = FibonacciExecutor::new();
    let module_executor = ModuleExecutor::new();
    let executors: Vec<&dyn Executor<Goldilocks>> = vec![&fibonacci_executor, &module_executor];

    let prover_builder = MockedProverBuilder::<Goldilocks>::new();

    // let mut proofman = match ProofManager::new(proofman_config, executors, prover_builder, false) {
    //     Ok(proofman) => proofman,
    //     Err(err) => {
    //         println!("Error: {:?}", err);
    //         return;
    //     }
    // };

    // let now = std::time::Instant::now();
    // let proof = proofman.prove(Some(public_inputs.into()));
    // if let Err(err) = proof {
    //     println!("Error: {}", err);
    // }

    // debug!("Proof generated in {} ms", now.elapsed().as_millis());
}
