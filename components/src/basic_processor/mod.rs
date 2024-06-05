mod common;
mod config;
mod rom_link;
mod trace;
mod basic_processor;
mod registers;

pub use common::*;
pub use config::*;
pub use rom_link::*;
pub use trace::*;
pub use basic_processor::*;
pub use registers::*;

#[cfg(test)]
mod tests {

    use goldilocks::Goldilocks;

    use super::{BasicProcessor, BasicProcessorConfig, BasicProcessorTrace};

    #[test]
    fn basic_processor_new_works() {
        let config: BasicProcessorConfig = BasicProcessorConfig {
            rom_json_path: "test".to_string(),
        };
        let processor = BasicProcessor::<Goldilocks>::new(config);
    }
}
