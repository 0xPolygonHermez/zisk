// use std::collections::HashMap;

// use log::debug;

// use goldilocks::{Goldilocks, AbstractField};

// mod executor_fibo;
// use executor_fibo::FibonacciExecutor;

// mod executor_module;
// use executor_module::ModuleExecutor;

// use proofman::{executor::Executor, proof_manager::ProofManager};

// use proofman::proof_manager_config::ProofManConfig;

// use serde::{Deserialize, Serialize};
// use serde_json;

// use clap::Parser;
// use proofman_cli::commands::prove::ProveCmd;
// use stark::stark_buffer_manager::StarkBufferManager;
// use stark::stark_prover_builder::StarkProverBuilder;
// use stark::stark_prover_settings::StarkProverSettings;

// use pilout::pilout_proxy::PilOutProxy;
// use starks_lib_c::*;

// #[derive(Debug, Serialize, Deserialize)]
// pub struct FibVPublicInputs<T> {
//     a: T,
//     b: T,
//     module: T,
// }

// impl FibVPublicInputs<u64> {
//     pub fn new(json: &str) -> FibVPublicInputs<Goldilocks> {
//         let data: Result<FibVPublicInputs<u64>, _> = serde_json::from_str(&json);

//         match data {
//             Ok(data) => FibVPublicInputs {
//                 a: Goldilocks::from_canonical_u64(data.a),
//                 b: Goldilocks::from_canonical_u64(data.b),
//                 module: Goldilocks::from_canonical_u64(data.module),
//             },
//             Err(e) => panic!("Error parsing settings file: {}", e),
//         }
//     }
// }

// impl<T> Into<Vec<T>> for FibVPublicInputs<T> {
//     fn into(self) -> Vec<T> {
//         vec![self.a, self.b, self.module]
//     }
// }

use pil2_components::{BasicProcessor, BasicProcessorConfig, BasicProcessorTrace, BasicProcessorTrace2};
use goldilocks::Goldilocks;

fn main() {
    let config = BasicProcessorConfig { rom_json_path: "examples/zkevm_basic/data/rom.json".to_string() };
    let n = 16;
    let mut trace = BasicProcessorTrace::<Goldilocks>::new(n);
    let mut trace2 = BasicProcessorTrace2::<Goldilocks>::new(n);

    let mut basic_zkevm_processor = BasicProcessor::<Goldilocks>::new(config, &mut trace, &mut trace2);

    basic_zkevm_processor.execute();
    // env_logger::builder().format_timestamp(None).format_target(false).filter_level(log::LevelFilter::Trace).init();

    // let arguments: ProveCmd = ProveCmd::parse();

    // let config_json = std::fs::read_to_string(arguments.config).expect("Failed to read file");
    // let proofman_config = ProofManConfig::parse_input_json(&config_json);

    // //read public inputs file
    // let public_inputs_filename = arguments.public_inputs.as_ref().unwrap();
    // let public_inputs_json = std::fs::read_to_string(&public_inputs_filename).unwrap_or_else(|err| {
    //     println!("Error reading public inputs file '{}': {}", &public_inputs_filename.display(), err);
    //     std::process::exit(1);
    // });
    // let public_inputs = FibVPublicInputs::new(&public_inputs_json);

    // let mut buffer_manager = StarkBufferManager::<Goldilocks>::new();

    // let pilout = PilOutProxy::new(proofman_config.get_pilout(), false).unwrap();

    // let mut prover_builders = HashMap::new();

    // let mut p_steps_vec = Vec::new();

    // for subproof in pilout.pilout.subproofs.iter() {
    //     debug!("Subproof '{}'", subproof.name());
    //     for air in subproof.airs.iter() {
    //         debug!("Air      '{}'", air.name());

    //         let air_name = air.name();

    //         let stark_config = StarkProverSettings {
    //             current_path: "examples/fibv/data/run.config.json".to_owned(),
    //             const_pols_filename: format!("examples/fibv/config/{}.const", air_name),
    //             map_const_pols_file: false,
    //             const_tree_filename: format!("examples/fibv/config/{}.consttree", air_name),
    //             stark_info_filename: format!("examples/fibv/config/{}.starkinfo.json", air_name),
    //             verkey_filename: format!("examples/fibv/config/{}.verkey.json", air_name),
    //             chelpers_filename: format!("examples/fibv/config/{}.chelpers.bin", air_name),
    //         };

    //         check_file_exists(&stark_config.current_path);
    //         check_file_exists(&stark_config.const_pols_filename);
    //         check_file_exists(&stark_config.const_tree_filename);
    //         check_file_exists(&stark_config.stark_info_filename);
    //         check_file_exists(&stark_config.verkey_filename);
    //         check_file_exists(&stark_config.chelpers_filename);

    //         let p_steps = generic_steps_new_c();
    //         p_steps_vec.push(p_steps);

    //         let prover_builder = StarkProverBuilder::new(stark_config.clone(), p_steps);
    //         prover_builders.insert(air.name().to_owned(), prover_builder);

    //         buffer_manager.insert_item(
    //             subproof.name(),
    //             &stark_config.stark_info_filename,
    //             &stark_config.chelpers_filename,
    //         );
    //     }
    // }

    // let fibonacci_executor = FibonacciExecutor::new();
    // let module_executor = ModuleExecutor::new();
    // let executors: Vec<&dyn Executor<Goldilocks>> = vec![&fibonacci_executor, &module_executor];

    // let mut proofman =
    //     ProofManager::new(proofman_config, executors, prover_builders, Some(Box::new(buffer_manager)), false)
    //         .unwrap_or_else(|err| {
    //             println!("Error: {}", err);
    //             std::process::exit(1);
    //         });

    // let _proof = proofman.prove(Some(public_inputs.into())).unwrap_or_else(|err| {
    //     println!("Error: {}", err);
    //     std::process::exit(1);
    // });

    // // Free memory p_steeps_vec
    // for p_steps in p_steps_vec {
    //     generic_steps_free_c(p_steps);
    // }
}

// fn check_file_exists(filename: &str) {
//     if !std::path::Path::new(filename).exists() {
//         println!("Error: File '{}' not found", filename);
//         std::process::exit(1);
//     }
// }
