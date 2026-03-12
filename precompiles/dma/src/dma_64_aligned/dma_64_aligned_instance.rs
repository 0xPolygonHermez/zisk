//! The `Dma64AlignedInstance` module defines an instance to perform the witness computation
//! for the Dma State Machine.
//!
//! It manages collected inputs and interacts with the `DmaSM` to compute witnesses for
//! execution plans.

#[cfg(feature = "save_dma_collectors")]
use crate::save_dma_collectors;
#[cfg(feature = "save_dma_inputs")]
use crate::Dma64AlignedInput;
use crate::{
    Dma64AlignedCollector, Dma64AlignedModule, DmaCheckPoint, DMA_64_ALIGNED_INPUTCPY_OPS_BY_ROW,
    DMA_64_ALIGNED_MEMCPY_OPS_BY_ROW, DMA_64_ALIGNED_MEMSET_OPS_BY_ROW,
    DMA_64_ALIGNED_MEM_OPS_BY_ROW, DMA_64_ALIGNED_OPS_BY_ROW,
};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use std::sync::Arc;
use zisk_common::ChunkId;
use zisk_common::StatsType;
use zisk_common::{BusDevice, CheckPoint, Instance, InstanceCtx, InstanceType, PayloadType};
use zisk_pil::{
    Dma64AlignedInputCpyTrace, Dma64AlignedMemCpyTrace, Dma64AlignedMemSetTrace,
    Dma64AlignedMemTrace, Dma64AlignedTrace,
};

pub const F_SEL_MEMCPY: u64 = 1;
pub const F_SEL_MEMCMP: u64 = 2;
pub const F_SEL_INPUTCPY: u64 = 4;
pub const F_SEL_MEMSET: u64 = 8;

/// The `Dma64AlignedInstance` struct represents an instance for the Dma State Machine.
///
/// It encapsulates the `Dma64AlignedSM` and its associated context, and it processes input data
/// to compute witnesses for the Dma64Aligned State Machine.
pub struct Dma64AlignedInstance<F: PrimeField64> {
    /// Dma state machine.
    module: Arc<dyn Dma64AlignedModule<F>>,

    /// Instance context.
    ictx: InstanceCtx,

    /// Flag to define that it's last segment
    is_last_segment: bool,
}

impl<F: PrimeField64> Dma64AlignedInstance<F> {
    /// Creates a new `Dma64AlignedInstance`.
    ///
    /// # Arguments
    /// * `module` - An `Arc`-wrapped reference to the Dma 64 Aligned Module.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `Dma64AlignedInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(module: Arc<dyn Dma64AlignedModule<F>>, ictx: InstanceCtx) -> Self {
        let is_last_segment = {
            let meta = ictx.plan.meta.as_ref().unwrap();
            let checkpoint = meta.downcast_ref::<DmaCheckPoint>().unwrap();
            checkpoint.is_last_segment
        };
        Self { module, ictx, is_last_segment }
    }

    pub fn build_dma_collector(&self, chunk_id: ChunkId) -> Dma64AlignedCollector {
        let ops_by_row = match self.ictx.plan.air_id {
            Dma64AlignedTrace::<F>::AIR_ID => DMA_64_ALIGNED_OPS_BY_ROW,
            Dma64AlignedMemCpyTrace::<F>::AIR_ID => DMA_64_ALIGNED_MEMCPY_OPS_BY_ROW,
            Dma64AlignedInputCpyTrace::<F>::AIR_ID => DMA_64_ALIGNED_INPUTCPY_OPS_BY_ROW,
            Dma64AlignedMemSetTrace::<F>::AIR_ID => DMA_64_ALIGNED_MEMSET_OPS_BY_ROW,
            Dma64AlignedMemTrace::<F>::AIR_ID => DMA_64_ALIGNED_MEM_OPS_BY_ROW,
            _ => panic!("Dma64AlignedInstance: Unsupported air_id: {:?}", self.ictx.plan.air_id),
        };

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<DmaCheckPoint>().unwrap();
        let (num_inputs, collect_counters) = collect_info.chunks[&chunk_id];
        Dma64AlignedCollector::new(
            chunk_id,
            num_inputs,
            collect_counters,
            ops_by_row,
            Some(chunk_id) == collect_info.last_chunk,
        )
    }
}

impl<F: PrimeField64> Instance<F> for Dma64AlignedInstance<F> {
    /// Computes the witness for the Dma execution plan.
    ///
    /// This method leverages the `Dma64AlignedSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        #[cfg(feature = "save_dma_collectors")]
        let (debug, inputs): (Vec<_>, Vec<_>) = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<Dma64AlignedCollector>().unwrap().take_debug_inputs()
            })
            .unzip();
        #[cfg(not(feature = "save_dma_collectors"))]
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<Dma64AlignedCollector>().unwrap().take_inputs()
            })
            .collect();

        let segment_id = self.ictx.plan.segment_id.unwrap();

        #[cfg(feature = "save_dma_collectors")]
        save_dma_collectors(
            &format!("{}_collector_{segment_id:04}.txt", self.module.get_name()),
            debug,
        )?;

        #[cfg(feature = "save_dma_inputs")]
        Dma64AlignedInput::save_debug_info(
            &format!("{}_inputs_{segment_id:04}.txt", self.module.get_name()),
            &inputs,
        )?;

        Ok(Some(self.module.compute_witness(
            &inputs,
            segment_id,
            self.is_last_segment,
            trace_buffer,
        )?))
    }

    /// Retrieves the checkpoint associated with this instance.
    ///
    /// # Returns
    /// A `CheckPoint` object representing the checkpoint of the execution plan.
    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    /// Retrieves the type of this instance.
    ///
    /// # Returns
    /// An `InstanceType` representing the type of this instance (`InstanceType::Instance`).
    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn stats_type(&self) -> StatsType {
        StatsType::Precompiled
    }

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(self.build_dma_collector(chunk_id)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
