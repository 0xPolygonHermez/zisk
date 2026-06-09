//! Collector for the built-in SMs and the per-chunk air-id dispatch that fills them.

use crate::error::{ExecutorError, ExecutorResult};

use fields::PrimeField64;
use precomp_dma::{
    Dma64AlignedCollector, Dma64AlignedInstance, DmaCollector, DmaCounterInputGen, DmaInstance,
    DmaPrePostCollector, DmaPrePostInstance, DmaUnalignedCollector, DmaUnalignedInstance,
};
use sm_arith::{ArithCounterInputGen, ArithFullInstance, ArithInstanceCollector};
use sm_binary::{
    BinaryAddCollector, BinaryAddInstance, BinaryBasicCollector, BinaryBasicInstance,
    BinaryExtensionCollector, BinaryExtensionInstance,
};
use sm_mem::{
    MemAlignByteInstance, MemAlignCollector, MemAlignInstance, MemAlignReadByteInstance,
    MemAlignWriteByteInstance, MemModuleCollector, MemModuleInstance,
};
use sm_rom::{RomCollector, RomInstance};
use zisk_common::BusDeviceMode;
use zisk_common::{ChunkId, Instance};
use zisk_pil::{
    ARITH_AIR_IDS, BINARY_ADD_AIR_IDS, BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS,
    DMA_64_ALIGNED_AIR_IDS, DMA_64_ALIGNED_INPUT_CPY_AIR_IDS, DMA_64_ALIGNED_MEM_AIR_IDS,
    DMA_64_ALIGNED_MEM_CPY_AIR_IDS, DMA_64_ALIGNED_MEM_SET_AIR_IDS, DMA_AIR_IDS,
    DMA_INPUT_CPY_AIR_IDS, DMA_MEM_CPY_AIR_IDS, DMA_PRE_POST_AIR_IDS,
    DMA_PRE_POST_INPUT_CPY_AIR_IDS, DMA_PRE_POST_MEM_CPY_AIR_IDS, DMA_UNALIGNED_AIR_IDS,
    INPUT_DATA_AIR_IDS, MEM_AIR_IDS, MEM_ALIGN_AIR_IDS, MEM_ALIGN_BYTE_AIR_IDS,
    MEM_ALIGN_READ_BYTE_AIR_IDS, MEM_ALIGN_WRITE_BYTE_AIR_IDS, ROM_AIR_IDS, ROM_DATA_AIR_IDS,
};

/// Collector for the built-in SMs.
pub struct BuiltinCollectors<F: PrimeField64> {
    /// ROM operation collectors.
    pub rom: Vec<(usize, RomCollector)>,

    /// Memory-related collectors.
    pub mem: Vec<(usize, MemModuleCollector)>,
    /// Memory alignment-related collectors.
    pub mem_align: Vec<(usize, MemAlignCollector)>,

    /// Binary basic operation collectors.
    pub binary_basic: Vec<(usize, BinaryBasicCollector<F>)>,
    /// Binary add operation collectors.
    pub binary_add: Vec<(usize, BinaryAddCollector<F>)>,
    /// Binary extension operation collectors.
    pub binary_extension: Vec<(usize, BinaryExtensionCollector<F>)>,

    /// Arithmetic operation collectors.
    pub arith: Vec<(usize, ArithInstanceCollector<F>)>,
    /// Arithmetic input generator.
    pub arith_inputs_generator: ArithCounterInputGen,

    /// DMA-related collectors.
    pub dma: Vec<(usize, DmaCollector)>,
    /// DMA pre/post operation collectors.
    pub dma_pre_post: Vec<(usize, DmaPrePostCollector)>,
    /// DMA 64-bit aligned operation collectors.
    pub dma_64_aligned: Vec<(usize, Dma64AlignedCollector)>,
    /// DMA unaligned operation collectors.
    pub dma_unaligned: Vec<(usize, DmaUnalignedCollector)>,
    /// DMA input generator.
    pub dma_inputs_generator: DmaCounterInputGen,
}

impl<F: PrimeField64> BuiltinCollectors<F> {
    /// Builds the input generators. Collector vecs start empty.
    pub(crate) fn new() -> Self {
        Self {
            rom: Vec::new(),
            mem: Vec::new(),
            mem_align: Vec::new(),
            binary_basic: Vec::new(),
            binary_add: Vec::new(),
            binary_extension: Vec::new(),
            arith: Vec::new(),
            arith_inputs_generator: ArithCounterInputGen::new(BusDeviceMode::InputGenerator),
            dma: Vec::new(),
            dma_pre_post: Vec::new(),
            dma_64_aligned: Vec::new(),
            dma_unaligned: Vec::new(),
            dma_inputs_generator: DmaCounterInputGen::new(BusDeviceMode::InputGenerator),
        }
    }

    /// Per-chunk air-id dispatch.
    ///
    /// # Returns
    /// `Ok(true)` if `air_id` matched a built-in SM and the corresponding collector was pushed
    /// `Ok(false)` if `air_id` did not match any built-in SM
    /// `Err` if `air_id` matched a built-in SM but the downcast failed
    pub(crate) fn try_push_collector(
        &mut self,
        air_id: usize,
        secn_instance: &dyn Instance<F>,
        chunk_id: ChunkId,
        global_idx: usize,
        mem_sections: &dyn zisk_core::MemDataSection,
    ) -> ExecutorResult<bool> {
        if self.try_push_rom(air_id, secn_instance, chunk_id, global_idx)? {
            return Ok(true);
        }
        if self.try_push_mem(air_id, secn_instance, chunk_id, global_idx, mem_sections)? {
            return Ok(true);
        }
        if self.try_push_binary(air_id, secn_instance, chunk_id, global_idx)? {
            return Ok(true);
        }
        if self.try_push_arith(air_id, secn_instance, chunk_id, global_idx)? {
            return Ok(true);
        }
        if self.try_push_dma(air_id, secn_instance, chunk_id, global_idx, mem_sections)? {
            return Ok(true);
        }
        Ok(false)
    }

    #[inline]
    fn try_push_rom(
        &mut self,
        air_id: usize,
        secn: &dyn Instance<F>,
        chunk: ChunkId,
        gid: usize,
    ) -> ExecutorResult<bool> {
        if air_id != ROM_AIR_IDS[0] {
            return Ok(false);
        }
        let inst = downcast::<F, RomInstance>(secn, air_id, gid, "RomInstance")?;
        if let Some(collector) = inst.build_rom_collector(chunk) {
            self.rom.push((gid, collector));
        }
        Ok(true)
    }

    #[inline]
    fn try_push_mem(
        &mut self,
        air_id: usize,
        secn: &dyn Instance<F>,
        chunk: ChunkId,
        gid: usize,
        mem_sections: &dyn zisk_core::MemDataSection,
    ) -> ExecutorResult<bool> {
        match air_id {
            id if id == MEM_AIR_IDS[0]
                || id == INPUT_DATA_AIR_IDS[0]
                || id == ROM_DATA_AIR_IDS[0] =>
            {
                let inst =
                    downcast::<F, MemModuleInstance<F>>(secn, air_id, gid, "MemModuleInstance")?;
                self.mem.push((gid, inst.build_mem_collector(chunk, mem_sections)));
                Ok(true)
            }
            id if id == MEM_ALIGN_AIR_IDS[0] => {
                let inst =
                    downcast::<F, MemAlignInstance<F>>(secn, air_id, gid, "MemAlignInstance")?;
                self.mem_align.push((gid, inst.build_mem_align_collector(chunk, mem_sections)));
                Ok(true)
            }
            id if id == MEM_ALIGN_BYTE_AIR_IDS[0] => {
                let inst = downcast::<F, MemAlignByteInstance<F>>(
                    secn,
                    air_id,
                    gid,
                    "MemAlignByteInstance",
                )?;
                self.mem_align.push((gid, inst.build_mem_align_byte_collector(chunk, mem_sections)));
                Ok(true)
            }
            id if id == MEM_ALIGN_READ_BYTE_AIR_IDS[0] => {
                let inst = downcast::<F, MemAlignReadByteInstance<F>>(
                    secn,
                    air_id,
                    gid,
                    "MemAlignReadByteInstance",
                )?;
                self.mem_align
                    .push((gid, inst.build_mem_align_read_byte_collector(chunk, mem_sections)));
                Ok(true)
            }
            id if id == MEM_ALIGN_WRITE_BYTE_AIR_IDS[0] => {
                let inst = downcast::<F, MemAlignWriteByteInstance<F>>(
                    secn,
                    air_id,
                    gid,
                    "MemAlignWriteByteInstance",
                )?;
                self.mem_align
                    .push((gid, inst.build_mem_align_write_byte_collector(chunk, mem_sections)));
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    #[inline]
    fn try_push_binary(
        &mut self,
        air_id: usize,
        secn: &dyn Instance<F>,
        chunk: ChunkId,
        gid: usize,
    ) -> ExecutorResult<bool> {
        match air_id {
            id if id == BINARY_AIR_IDS[0] => {
                let inst = downcast::<F, BinaryBasicInstance<F>>(
                    secn,
                    air_id,
                    gid,
                    "BinaryBasicInstance",
                )?;
                self.binary_basic.push((gid, inst.build_binary_basic_collector(chunk)));
                Ok(true)
            }
            id if id == BINARY_ADD_AIR_IDS[0] => {
                let inst =
                    downcast::<F, BinaryAddInstance<F>>(secn, air_id, gid, "BinaryAddInstance")?;
                self.binary_add.push((gid, inst.build_binary_add_collector(chunk)));
                Ok(true)
            }
            id if id == BINARY_EXTENSION_AIR_IDS[0] => {
                let inst = downcast::<F, BinaryExtensionInstance<F>>(
                    secn,
                    air_id,
                    gid,
                    "BinaryExtensionInstance",
                )?;
                self.binary_extension.push((gid, inst.build_binary_extension_collector(chunk)));
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    #[inline]
    fn try_push_arith(
        &mut self,
        air_id: usize,
        secn: &dyn Instance<F>,
        chunk: ChunkId,
        gid: usize,
    ) -> ExecutorResult<bool> {
        if air_id != ARITH_AIR_IDS[0] {
            return Ok(false);
        }
        let inst = downcast::<F, ArithFullInstance<F>>(secn, air_id, gid, "ArithFullInstance")?;
        self.arith.push((gid, inst.build_arith_collector(chunk)));
        Ok(true)
    }

    #[inline]
    fn try_push_dma(
        &mut self,
        air_id: usize,
        secn: &dyn Instance<F>,
        chunk: ChunkId,
        gid: usize,
        mem_sections: &dyn zisk_core::MemDataSection,
    ) -> ExecutorResult<bool> {
        match air_id {
            id if id == DMA_AIR_IDS[0]
                || id == DMA_MEM_CPY_AIR_IDS[0]
                || id == DMA_INPUT_CPY_AIR_IDS[0] =>
            {
                let inst = downcast::<F, DmaInstance<F>>(secn, air_id, gid, "DmaInstance")?;
                self.dma.push((gid, inst.build_dma_collector(chunk, mem_sections)));
                Ok(true)
            }
            id if id == DMA_PRE_POST_AIR_IDS[0]
                || id == DMA_PRE_POST_MEM_CPY_AIR_IDS[0]
                || id == DMA_PRE_POST_INPUT_CPY_AIR_IDS[0] =>
            {
                let inst =
                    downcast::<F, DmaPrePostInstance<F>>(secn, air_id, gid, "DmaPrePostInstance")?;
                self.dma_pre_post.push((gid, inst.build_dma_collector(chunk, mem_sections)));
                Ok(true)
            }
            id if id == DMA_64_ALIGNED_AIR_IDS[0]
                || id == DMA_64_ALIGNED_MEM_CPY_AIR_IDS[0]
                || id == DMA_64_ALIGNED_INPUT_CPY_AIR_IDS[0]
                || id == DMA_64_ALIGNED_MEM_SET_AIR_IDS[0]
                || id == DMA_64_ALIGNED_MEM_AIR_IDS[0] =>
            {
                let inst = downcast::<F, Dma64AlignedInstance<F>>(
                    secn,
                    air_id,
                    gid,
                    "Dma64AlignedInstance",
                )?;
                self.dma_64_aligned.push((gid, inst.build_dma_collector(chunk, mem_sections)));
                Ok(true)
            }
            id if id == DMA_UNALIGNED_AIR_IDS[0] => {
                let inst = downcast::<F, DmaUnalignedInstance<F>>(
                    secn,
                    air_id,
                    gid,
                    "DmaUnalignedInstance",
                )?;
                self.dma_unaligned.push((gid, inst.build_dma_collector(chunk, mem_sections)));
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

/// Downcasts `secn` to `T`, mapping a failed downcast to an
/// `InstanceTypeMismatch` error tagged with the expected type name.
#[inline]
fn downcast<'a, F: PrimeField64, T: 'static>(
    secn: &'a dyn Instance<F>,
    air_id: usize,
    global_id: usize,
    expected: &'static str,
) -> ExecutorResult<&'a T> {
    secn.as_any().downcast_ref::<T>().ok_or(ExecutorError::InstanceTypeMismatch {
        global_id,
        air_id,
        expected,
    })
}
