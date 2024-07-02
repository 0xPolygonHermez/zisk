mod arith;
mod binary;
#[allow(special_module_name)]
mod main;
mod mem;
mod zisk_processor;

pub use zisk_processor::*;

use std::collections::HashMap;

use proofman_mock::AirWitnessComputation;

pub fn get_stdlib_wc<T, I>() -> HashMap<String, Box<dyn AirWitnessComputation<T, I>>> {
    let stdlib_modules: HashMap<String, Box<dyn AirWitnessComputation<T, I>>> = HashMap::new();

    // stdlib_modules.insert("logup".to_string(), Box::new(Logup::new()));

    stdlib_modules
}
