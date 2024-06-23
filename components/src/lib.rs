// mod memory;
// mod basic_processor;
// mod component;
// mod memory;
// mod register;
// mod rom;

// // pub use memory::*;
// pub use basic_processor::*;
// pub use rom::*;
// pub use component::*;
// pub use memory::*;

use std::collections::HashMap;

use proofman_mock::AirWitnessComputation;

pub fn get_stdlib_wc<T, I>() -> HashMap<String, Box<dyn AirWitnessComputation<T, I>>> {
    let stdlib_modules: HashMap<String, Box<dyn AirWitnessComputation<T, I>>> = HashMap::new();

    // stdlib_modules.insert("logup".to_string(), Box::new(Logup::new()));

    stdlib_modules

}

#[cfg(test)]
mod tests {

    pub fn add(left: usize, right: usize) -> usize {
        left + right
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
