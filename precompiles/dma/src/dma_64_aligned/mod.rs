#[allow(clippy::module_inception)]
mod dma_64_aligned;
mod dma_64_aligned_collector;
mod dma_64_aligned_input;
mod dma_64_aligned_inputcpy;
mod dma_64_aligned_instance;
mod dma_64_aligned_mem;
mod dma_64_aligned_memcpy;
mod dma_64_aligned_memset;
mod dma_64_aligned_module;

pub use dma_64_aligned::*;
pub use dma_64_aligned_collector::*;
pub use dma_64_aligned_input::*;
pub use dma_64_aligned_inputcpy::*;
pub use dma_64_aligned_instance::*;
pub use dma_64_aligned_mem::*;
pub use dma_64_aligned_memcpy::*;
pub use dma_64_aligned_memset::*;
pub use dma_64_aligned_module::*;
