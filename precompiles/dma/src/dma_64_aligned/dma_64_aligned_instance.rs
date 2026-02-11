//! The `Dma64AlignedInstance` module defines an instance to perform the witness computation
//! for the Dma State Machine.
//!
//! It manages collected inputs and interacts with the `DmaSM` to compute witnesses for
//! execution plans.

use crate::{Dma64AlignedCollector, Dma64AlignedSM, DmaCheckPoint};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use std::sync::Arc;
use zisk_common::ChunkId;
use zisk_common::StatsType;
use zisk_common::{BusDevice, CheckPoint, Instance, InstanceCtx, InstanceType, PayloadType};
use zisk_pil::Dma64AlignedTrace;

/// The `Dma64AlignedInstance` struct represents an instance for the Dma State Machine.
///
/// It encapsulates the `Dma64AlignedSM` and its associated context, and it processes input data
/// to compute witnesses for the Dma64Aligned State Machine.
pub struct Dma64AlignedInstance<F: PrimeField64> {
    /// Dma state machine.
    dma_64_aligned_sm: Arc<Dma64AlignedSM<F>>,

    /// Instance context.
    ictx: InstanceCtx,

    /// Flag to define that it's last segment
    is_last_segment: bool,
}

impl<F: PrimeField64> Dma64AlignedInstance<F> {
    /// Creates a new `Dma64AlignedInstance`.
    ///
    /// # Arguments
    /// * `dma_64_aligned_sm` - An `Arc`-wrapped reference to the Dma 64 Aligned State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `Dma64AlignedInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(dma_64_aligned_sm: Arc<Dma64AlignedSM<F>>, ictx: InstanceCtx) -> Self {
        let is_last_segment = {
            let meta = ictx.plan.meta.as_ref().unwrap();
            let checkpoint = meta.downcast_ref::<DmaCheckPoint>().unwrap();
            checkpoint.is_last_segment
        };
        Self { dma_64_aligned_sm, ictx, is_last_segment }
    }

    pub fn build_dma_collector(&self, chunk_id: ChunkId) -> Dma64AlignedCollector {
        assert_eq!(
            self.ictx.plan.air_id,
            Dma64AlignedTrace::<F>::AIR_ID,
            "Dma64AlignedInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<DmaCheckPoint>().unwrap();
        let (num_inputs, collect_counter) = collect_info.chunks[&chunk_id];
        Dma64AlignedCollector::new(
            num_inputs,
            collect_counter,
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
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector.as_any().downcast::<Dma64AlignedCollector>().unwrap().inputs
            })
            .collect();
        // Extract segment id from instance context
        let segment_id = self.ictx.plan.segment_id.unwrap();

        Ok(Some(self.dma_64_aligned_sm.compute_witness(
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
        assert_eq!(
            self.ictx.plan.air_id,
            Dma64AlignedTrace::<F>::AIR_ID,
            "Dma64AlignedInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<DmaCheckPoint>().unwrap();
        let (num_inputs, collect_counter) = collect_info.chunks[&chunk_id];
        Some(Box::new(Dma64AlignedCollector::new(
            num_inputs,
            collect_counter,
            Some(chunk_id) == collect_info.last_chunk,
        )))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
