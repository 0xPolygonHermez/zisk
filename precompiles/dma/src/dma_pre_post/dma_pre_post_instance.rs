//! The `DmaPrePostInstance` module defines an instance to perform the witness computation
//! for the DmaPrePost State Machine.
//!
//! It manages collected inputs and interacts with the `DmaPrePostSM` to compute witnesses for
//! execution plans.

use crate::{DmaCheckPoint, DmaPrePostCollector, DmaPrePostSM};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use std::sync::Arc;
use zisk_common::ChunkId;
use zisk_common::{BusDevice, CheckPoint, Instance, InstanceCtx, InstanceType, PayloadType};
use zisk_pil::DmaPrePostTrace;

/// The `DmaPrePostInstance` struct represents an instance for the DmaPrePost State Machine.
///
/// It encapsulates the `DmaPrePostSM` and its associated context, and it processes input data
/// to compute witnesses for the DmaPrePost State Machine.
pub struct DmaPrePostInstance<F: PrimeField64> {
    /// DmaPrePost State machine.
    dma_sm: Arc<DmaPrePostSM<F>>,

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
    pub fn new(dma_sm: Arc<DmaPrePostSM<F>>, ictx: InstanceCtx) -> Self {
        Self { dma_sm, ictx }
    }

    pub fn build_dma_collector(&self, chunk_id: ChunkId) -> DmaPrePostCollector {
        assert_eq!(
            self.ictx.plan.air_id,
            DmaPrePostTrace::<F>::AIR_ID,
            "DmaPrePostInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info: &DmaCheckPoint = meta.downcast_ref::<DmaCheckPoint>().unwrap();
        let (num_ops, collect_counter) = collect_info.chunks[&chunk_id];
        DmaPrePostCollector::new(num_ops, collect_counter)
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
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<DmaPrePostCollector>().unwrap().inputs
            })
            .collect();

        Ok(Some(self.dma_sm.compute_witness(&inputs, trace_buffer)?))
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

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            DmaPrePostTrace::<F>::AIR_ID,
            "DmaPrePostInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<DmaCheckPoint>().unwrap();
        let (num_ops, collect_counter) = collect_info.chunks[&chunk_id];
        Some(Box::new(DmaPrePostCollector::new(num_ops, collect_counter)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
