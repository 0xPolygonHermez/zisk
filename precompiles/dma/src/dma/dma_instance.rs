//! The `DmaInstance` module defines an instance to perform the witness computation
//! for the Dma State Machine.
//!
//! It manages collected inputs and interacts with the `DmaSM` to compute witnesses for
//! execution plans.

use crate::dma::dma_collector::DmaCollector;
use crate::{DmaCheckPoint, DmaSM};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use std::sync::Arc;
use zisk_common::ChunkId;
use zisk_common::StatsType;
use zisk_common::{BusDevice, CheckPoint, Instance, InstanceCtx, InstanceType, PayloadType};
use zisk_pil::DmaTrace;

/// The `DmaInstance` struct represents an instance for the Dma State Machine.
///
/// It encapsulates the `DmaSM` and its associated context, and it processes input data
/// to compute witnesses for the Dma State Machine.
pub struct DmaInstance<F: PrimeField64> {
    /// Dma state machine.
    dma_sm: Arc<DmaSM<F>>,

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
    pub fn new(dma_sm: Arc<DmaSM<F>>, ictx: InstanceCtx) -> Self {
        Self { dma_sm, ictx }
    }

    pub fn build_dma_collector(&self, chunk_id: ChunkId) -> DmaCollector {
        assert_eq!(
            self.ictx.plan.air_id,
            DmaTrace::<F>::AIR_ID,
            "DmaInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<DmaCheckPoint>().unwrap();
        let (num_ops, collect_counter) = collect_info.chunks[&chunk_id];
        DmaCollector::new(num_ops, collect_counter)
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
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| collector.as_any().downcast::<DmaCollector>().unwrap().inputs)
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
        Some(Box::new(DmaCollector::new(num_ops, collect_counter)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
