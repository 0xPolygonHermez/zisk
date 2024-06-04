mod basic_processor_trace;
mod basic_processor;

pub use basic_processor_trace::BasicProcessorTrace;
pub use basic_processor::*;

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
