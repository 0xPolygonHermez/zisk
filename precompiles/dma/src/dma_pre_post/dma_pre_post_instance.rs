//! The `DmaPrePostInstance` module defines an instance to perform the witness computation
//! for the DmaPrePost State Machine.
//!
//! It manages collected inputs and interacts with the `DmaPrePostSM` to compute witnesses for
//! execution plans.

#[cfg(feature = "save_dma_inputs")]
use crate::DmaPrePostInput;
use crate::{DmaCheckPoint, DmaPrePostCollector, DmaPrePostModule};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use std::sync::Arc;

use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType, StatsType,
};
use zisk_pil::{DmaPrePostInputCpyTrace, DmaPrePostMemCpyTrace, DmaPrePostTrace};

/// The `DmaPrePostInstance` struct represents an instance for the DmaPrePost State Machine.
///
/// It encapsulates the `DmaPrePostModule` and its associated context, and it processes input data
/// to compute witnesses for the DmaPrePost State Machine.
pub struct DmaPrePostInstance<F: PrimeField64> {
    /// DmaPrePost State machine.
    module: Arc<dyn DmaPrePostModule<F>>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> DmaPrePostInstance<F> {
    /// Creates a new `DmaPrePostInstance`.
    ///
    /// # Arguments
    /// * `dma_sm` - An `Arc`-wrapped reference to the DmaPrePost State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `DmaPrePostInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(module: Arc<dyn DmaPrePostModule<F>>, ictx: InstanceCtx) -> Self {
        Self { module, ictx }
    }

    pub fn build_dma_collector(&self, chunk_id: ChunkId) -> DmaPrePostCollector {
        debug_assert!(
            [
                DmaPrePostTrace::<F>::AIR_ID,
                DmaPrePostMemCpyTrace::<F>::AIR_ID,
                DmaPrePostInputCpyTrace::<F>::AIR_ID,
            ]
            .contains(&self.ictx.plan.air_id),
            "DmaPrePostInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info: &DmaCheckPoint = meta.downcast_ref::<DmaCheckPoint>().unwrap();
        let (num_ops, collect_counters) = collect_info.chunks[&chunk_id];
        DmaPrePostCollector::new(chunk_id, num_ops, collect_counters)
    }
}

impl<F: PrimeField64> Instance<F> for DmaPrePostInstance<F> {
    /// Computes the witness for the Dma execution plan.
    ///
    /// This method leverages the `DmaPrePostSM` to generate an `AirInstance` using the collected
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
                let collector = collector.as_any().downcast::<DmaPrePostCollector>().unwrap();
                (collector.get_debug_info(), collector.inputs)
            })
            .unzip();
        #[cfg(not(feature = "save_dma_collectors"))]
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<DmaPrePostCollector>().unwrap().inputs
            })
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
        DmaPrePostInput::dump_to_file(
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
            DmaPrePostTrace::<F>::AIR_ID,
            "DmaPrePostInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<DmaCheckPoint>().unwrap();
        let (num_ops, collect_counters) = collect_info.chunks[&chunk_id];
        Some(Box::new(DmaPrePostCollector::new(chunk_id, num_ops, collect_counters)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "save_dma_collectors")]
pub fn save_dma_collectors(filename: &str, debug: Vec<String>) -> std::io::Result<()> {
    use std::fs;

    let path = std::env::var("DEBUG_OUTPUT_PATH").unwrap_or_else(|_| "tmp/".to_string());
    let full_path = format!("{}{}", path, filename);

    fs::write(&full_path, debug.join("\n"))?;
    Ok(())
}
