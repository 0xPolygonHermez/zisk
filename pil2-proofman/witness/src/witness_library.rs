use std::{error::Error, sync::Arc};

use crate::WitnessManager;
use proofman_common::VerboseMode;

/// This is the type of the function that is used to load a witness library.
pub type WitnessLibInitFn<F> = fn(VerboseMode) -> Result<Box<dyn WitnessLibrary<F>>, Box<dyn Error>>;

pub trait WitnessLibrary<F> {
    fn register_witness(&mut self, wcm: Arc<WitnessManager<F>>);
}

#[macro_export]
macro_rules! witness_library {
    ($lib_name:ident, $field_type:ty) => {
        // Define the struct
        pub struct $lib_name;

        // Define the init_library function
        #[no_mangle]
        pub extern "Rust" fn init_library(
            verbose_mode: proofman_common::VerboseMode,
        ) -> Result<Box<dyn witness::WitnessLibrary<$field_type>>, Box<dyn std::error::Error>> {
            proofman_common::initialize_logger(verbose_mode);

            Ok(Box::new($lib_name))
        }
    };
}
