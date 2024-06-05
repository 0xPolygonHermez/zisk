mod basic_processor_common;
mod basic_processor_config;
mod basic_processor_rom_link;
mod basic_processor_trace;
mod basic_processor;
mod basic_processors_registers;

pub use basic_processor_common::*;
pub use basic_processor_config::*;
pub use basic_processor_rom_link::*;
pub use basic_processor_trace::*;
pub use basic_processor::*;
pub use basic_processors_registers::*;

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
