mod dma;
mod dma_64_aligned;
mod dma_bus_device;
mod dma_checkpoint;
mod dma_collect_counters;
mod dma_collector_routing_log;
mod dma_common;
mod dma_constants;
mod dma_gen_inputcpy_mem_inputs;
mod dma_gen_mem_inputs;
mod dma_gen_memcmp_mem_inputs;
mod dma_gen_memcpy_mem_inputs;
mod dma_gen_memset_mem_inputs;
mod dma_instance_info;
mod dma_instances_builder;
mod dma_manager;
mod dma_planner;
mod dma_pre_post;
mod dma_strategy;
mod dma_unaligned;

pub use dma::*;
pub use dma_64_aligned::*;
pub use dma_bus_device::*;
pub use dma_checkpoint::*;
pub use dma_collect_counters::*;
pub use dma_collector_routing_log::*;
pub use dma_common::*;
pub use dma_constants::*;
pub use dma_gen_inputcpy_mem_inputs::*;
pub use dma_gen_mem_inputs::*;
pub use dma_gen_memcmp_mem_inputs::*;
pub use dma_gen_memcpy_mem_inputs::*;
pub use dma_gen_memset_mem_inputs::*;
pub use dma_instance_info::*;
pub use dma_instances_builder::*;
pub use dma_manager::*;
pub use dma_planner::*;
pub use dma_pre_post::*;
pub use dma_strategy::*;
pub use dma_unaligned::*;

#[cfg(test)]
mod dma_mt_tests {
    use zisk_common::io::ZiskStdin;
    use zisk_test_artifacts::ELF_DMA_MT;

    /// Drives the `mt` DMA family (`dma_mtcpy`, `dma_mtcmp` and their extended variants) together
    /// with the `execute_advice` hint and the temporal-reference request from a guest: the guest
    /// asserts every result itself, so a wrong lowering or a wrong emulation surfaces as a failed
    /// emulation.
    ///
    /// The mt family has no state machine yet, so this covers the emulator only.
    #[test]
    fn dma_mt_tests() {
        ELF_DMA_MT.run_emulation(ZiskStdin::new(), None).expect("dma mt guest emulation failed");
    }
}
