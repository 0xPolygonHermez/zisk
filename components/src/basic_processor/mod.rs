mod basic_processor;
mod command;
mod config;
mod context;
mod registers;
mod rom_link;
mod trace;

use std::{cell::RefCell, rc::Rc};

pub use basic_processor::*;
pub use command::*;
pub use config::*;
pub use context::*;
pub use registers::*;
pub use rom_link::*;
pub use trace::*;

pub const CHUNKS: usize = 8;
pub const CHUNK_BITS: usize = 32;
pub const CHUNK_MASK: usize = (1 << CHUNK_BITS) - 1;

pub enum CallbackReturnType<T> {
    Single(T),
    Array([T; CHUNKS]),
}

pub enum TracePolEnum<T> {
    Single(Rc<RefCell<proofman::trace::trace_pol::TracePol<T>>>),
    Array(Rc<RefCell<proofman::trace::trace_pol::TracePol<[T; CHUNKS]>>>),
}

// pub enum CallbackType<T> {
//     // Single(Box<dyn Fn() -> T>),
//     // Array(Box<dyn Fn() -> [T; 8]>),
// }

#[cfg(test)]
mod tests {

    use goldilocks::Goldilocks;

    use super::{BasicProcessor, BasicProcessorConfig, BasicProcessorTrace};

    #[test]
    fn basic_processor_new_works() {
        let config: BasicProcessorConfig = BasicProcessorConfig { rom_json_path: "test".to_string() };

        let n = 16;
        let mut trace = BasicProcessorTrace::<Goldilocks>::new(n);

        let processor = BasicProcessor::<Goldilocks>::new(config, &mut trace);
    }
}
