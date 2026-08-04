//! The `BinaryAddHiInstance` module defines an specific instance to perform witness computations
//! for the packed add operations proven by the Binary Add Hi State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryAddHiSM` to compute witnesses for
//! execution plans.

use crate::{BinaryAddHiCollector, BinaryAddHiSM};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use std::{collections::HashMap, sync::Arc};
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, CollectSkipper, Instance, InstanceCtx, InstanceType,
    PayloadType,
};
use zisk_pil::{BinaryAddHiTrace, BinaryAddHiTraceRow, BinaryAddHiTraceRowPacked};

/// The `BinaryAddHiInstance` struct represents an instance for packed add witness computations.
///
/// It encapsulates the `BinaryAddHiSM` and its associated context, and it processes input data
/// to compute witnesses for the additions whose result fits in the low 32-bit limb.
pub struct BinaryAddHiInstance<F: PrimeField64> {
    /// Binary Add Hi state machine.
    binary_add_hi_sm: Arc<BinaryAddHiSM<F>>,

    /// Collect info for each chunk ID, containing the number of operations and a skipper for
    /// collection. One instance holds ADDS_X_ROW operations per row, so the count is in
    /// operations, not rows.
    collect_info: HashMap<ChunkId, (u64, bool, CollectSkipper)>,

    /// Instance context.
    ictx: InstanceCtx,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> BinaryAddHiInstance<F> {
    /// Creates a new `BinaryAddHiInstance`.
    ///
    /// # Arguments
    /// * `binary_add_hi_sm` - An `Arc`-wrapped reference to the Binary Add Hi State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    ///
    /// # Returns
    /// A new `BinaryAddHiInstance` initialized with the provided state machine and context.
    pub fn new(
        binary_add_hi_sm: Arc<BinaryAddHiSM<F>>,
        mut ictx: InstanceCtx,
        std: Arc<Std<F>>,
    ) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            BinaryAddHiTrace::<()>::AIR_ID,
            "BinaryAddHiInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, (u64, bool, CollectSkipper)>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { binary_add_hi_sm, collect_info, ictx, std }
    }

    pub fn build_binary_add_hi_collector(&self, chunk_id: ChunkId) -> BinaryAddHiCollector<F> {
        let (num_ops, force_execute_to_end, collect_skipper) = self.collect_info[&chunk_id];
        BinaryAddHiCollector::new(
            num_ops as usize,
            collect_skipper,
            force_execute_to_end,
            self.std.clone(),
        )
    }
}

impl<F: PrimeField64> Instance<F> for BinaryAddHiInstance<F> {
    /// Computes the witness for the packed add execution plan.
    ///
    /// This method leverages the `BinaryAddHiSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    /// * `_sctx` - The setup context, unused in this implementation.
    /// * `collectors` - A vector of input collectors to process and collect data for witness
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, collector)| {
                let collector = collector.as_any().downcast::<BinaryAddHiCollector<F>>().unwrap();
                collector.inputs
            })
            .collect();

        if packed {
            Ok(Some(
                self.binary_add_hi_sm
                    .compute_witness::<BinaryAddHiTraceRowPacked<F>>(&inputs, trace_buffer)?,
            ))
        } else {
            Ok(Some(
                self.binary_add_hi_sm
                    .compute_witness::<BinaryAddHiTraceRow<F>>(&inputs, trace_buffer)?,
            ))
        }
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
        StatsType::Opcodes
    }

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        let (num_ops, force_execute_to_end, collect_skipper) = self.collect_info[&chunk_id];
        Some(Box::new(BinaryAddHiCollector::new(
            num_ops as usize,
            collect_skipper,
            force_execute_to_end,
            self.std.clone(),
        )))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
