//! The `DmaInstance` module defines an instance to perform the witness computation
//! for the Dma State Machine.
//!
//! It manages collected inputs and interacts with the `DmaSM` to compute witnesses for
//! execution plans.

use crate::dma::dma_collector::DmaCollector;
#[cfg(feature = "save_dma_collectors")]
use crate::save_dma_collectors;
#[cfg(feature = "save_dma_inputs")]
use crate::DmaInput;
use crate::{DmaCheckPoint, DmaModule};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use std::sync::Arc;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType, StatsType,
};
use zisk_pil::{DmaInputCpyTrace, DmaMemCpyTrace, DmaTrace};

/// The `DmaInstance` struct represents an instance for the Dma State Machine.
///
/// It encapsulates the `DmaSM` and its associated context, and it processes input data
/// to compute witnesses for the Dma State Machine.
pub struct DmaInstance<F: PrimeField64> {
    /// Dma state machine.
    module: Arc<dyn DmaModule<F>>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> DmaInstance<F> {
    /// Creates a new `DmaInstance`.
    ///
    /// # Arguments
    /// * `dma_sm` - An `Arc`-wrapped reference to the Dma State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `DmaInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(module: Arc<dyn DmaModule<F>>, ictx: InstanceCtx) -> Self {
        Self { module, ictx }
    }

    pub fn build_dma_collector(&self, chunk_id: ChunkId) -> DmaCollector {
        debug_assert!(
            [DmaTrace::<F>::AIR_ID, DmaMemCpyTrace::<F>::AIR_ID, DmaInputCpyTrace::<F>::AIR_ID,]
                .contains(&self.ictx.plan.air_id),
            "DmaInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<DmaCheckPoint>().unwrap();
        let (num_ops, collect_counters) = collect_info.chunks[&chunk_id];
        DmaCollector::new(chunk_id, num_ops, collect_counters)
    }
}

impl<F: PrimeField64> Instance<F> for DmaInstance<F> {
    /// Computes the witness for the Dma execution plan.
    ///
    /// This method leverages the `DmaSM` to generate an `AirInstance` using the collected
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
                let collector = collector.as_any().downcast::<DmaCollector>().unwrap();
                (collector.get_debug_info(), collector.inputs)
            })
            .unzip();
        #[cfg(not(feature = "save_dma_collectors"))]
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| collector.as_any().downcast::<DmaCollector>().unwrap().inputs)
            .collect();

        #[cfg(any(feature = "save_dma_collectors", feature = "save_dma_inputs"))]
        let air_instance_id =
            _pctx.dctx_find_air_instance_id(self.ictx.plan.global_id.unwrap()).unwrap();

        #[cfg(feature = "save_dma_collectors")]
        save_dma_collectors(
            &format!("{}_collector_{air_instance_id:04}.txt", self.module.get_name()),
            debug,
        )?;

        #[cfg(feature = "save_dma_inputs")]
        DmaInput::dump_to_file(
            &inputs,
            &format!("{}_inputs_{air_instance_id:04}.txt", self.module.get_name()),
        )?;

        Ok(Some(self.module.compute_witness(&inputs, trace_buffer)?))
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
        assert_eq!(
            self.ictx.plan.air_id,
            DmaTrace::<F>::AIR_ID,
            "DmaInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<DmaCheckPoint>().unwrap();
        let (num_ops, collect_counter) = collect_info.chunks[&chunk_id];
        Some(Box::new(DmaCollector::new(chunk_id, num_ops, collect_counter)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
