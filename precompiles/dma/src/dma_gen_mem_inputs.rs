use precompiles_common::MemProcessor;
use zisk_common::OP;
use zisk_core::zisk_ops::ZiskOp;

use crate::generate_dma_inputcpy_mem_inputs;
use crate::generate_dma_memcmp_mem_inputs;
use crate::generate_dma_memcpy_mem_inputs;
use crate::generate_dma_memset_mem_inputs;
use crate::skip_dma_inputcpy_mem_inputs;
use crate::skip_dma_memcmp_mem_inputs;
use crate::skip_dma_memcpy_mem_inputs;
use crate::skip_dma_memset_mem_inputs;

pub fn generate_dma_mem_inputs<P: MemProcessor>(
    data: &[u64],
    data_ext: &[u64],
    _only_counters: bool,
    mem_processors: &mut P,
) {
    match data[OP] as u8 {
        ZiskOp::DMA_INPUTCPY => generate_dma_inputcpy_mem_inputs(data, data_ext, mem_processors),
        ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => {
            generate_dma_memcmp_mem_inputs(data, data_ext, mem_processors)
        }
        ZiskOp::DMA_XMEMSET => generate_dma_memset_mem_inputs(data, data_ext, mem_processors),
        ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY => {
            generate_dma_memcpy_mem_inputs(data, data_ext, mem_processors)
        }
        _ => panic!("Invalid op 0x{:02X}", data[OP]),
    }
}

pub fn skip_dma_mem_inputs<P: MemProcessor>(
    data: &[u64],
    _data_ext: &[u64],
    mem_processors: &mut P,
) -> bool {
    match data[OP] as u8 {
        ZiskOp::DMA_INPUTCPY => skip_dma_inputcpy_mem_inputs(data, mem_processors),
        ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => {
            skip_dma_memcmp_mem_inputs(data, mem_processors)
        }
        ZiskOp::DMA_XMEMSET => skip_dma_memset_mem_inputs(data, mem_processors),
        ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY => {
            skip_dma_memcpy_mem_inputs(data, mem_processors)
        }
        _ => panic!("Invalid op 0x{:02X}", data[OP]),
    }
}
