//! The `BinaryBasicInstance` module defines an instance to perform witness computations
//! for binary-related operations using the Binary Basic State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryBasicSM` to compute witnesses for
//! execution plans.

use crate::{BinaryBasicCollector, BinaryBasicSM, ChunkCollect, ADD_KINDS};
use pil2_std_lib::Std;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use std::{collections::HashMap, sync::Arc};
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};
use zisk_pil::{BinaryTrace, BinaryTraceRow, BinaryTraceRowPacked};

/// The `BinaryBasicInstance` struct represents an instance for binary-related witness computations.
///
/// It encapsulates the `BinaryBasicSM` and its associated context, and it processes input data
/// to compute witnesses for binary operations.
pub struct BinaryBasicInstance<F: PrimeField64> {
    /// Binary Basic state machine.
    binary_basic_sm: Arc<BinaryBasicSM<F>>,

    /// Instance context.
    ictx: InstanceCtx,

    /// What this instance takes from each chunk: a `(count, skip)` per kind of operation, plus the
    /// frequent operations it accounts for.
    collect_info: HashMap<ChunkId, ChunkCollect<ADD_KINDS>>,

    /// Standard library instance, providing common functionalities.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> BinaryBasicInstance<F> {
    /// Creates a new `BinaryBasicInstance`.
    ///
    /// # Arguments
    /// * `binary_basic_sm` - An `Arc`-wrapped reference to the Binary Basic State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    ///
    /// # Returns
    /// A new `BinaryBasicInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(
        binary_basic_sm: Arc<BinaryBasicSM<F>>,
        mut ictx: InstanceCtx,
        std: Arc<Std<F>>,
    ) -> Self {
        assert_eq!(
            ictx.plan.air_id,
            BinaryTrace::<()>::AIR_ID,
            "BinaryBasicInstance: Unsupported air_id: {:?}",
            ictx.plan.air_id
        );

        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, ChunkCollect<ADD_KINDS>>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { binary_basic_sm, ictx, collect_info, std }
    }

    pub fn build_binary_basic_collector(&self, chunk_id: ChunkId) -> BinaryBasicCollector<F> {
        BinaryBasicCollector::new(self.collect_info[&chunk_id], self.std.clone())
    }
}

impl<F: PrimeField64> Instance<F> for BinaryBasicInstance<F> {
    /// Computes the witness for the binary execution plan.
    ///
    /// This method leverages the `BinaryBasicSM` to generate an `AirInstance` using the collected
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
                let _collector = collector.as_any().downcast::<BinaryBasicCollector<F>>().unwrap();
                _collector.inputs
            })
            .collect();

        if packed {
            Ok(Some(
                self.binary_basic_sm
                    .compute_witness::<BinaryTraceRowPacked<F>>(&inputs, trace_buffer)?,
            ))
        } else {
            Ok(Some(
                self.binary_basic_sm.compute_witness::<BinaryTraceRow<F>>(&inputs, trace_buffer)?,
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
        Some(Box::new(BinaryBasicCollector::new(self.collect_info[&chunk_id], self.std.clone())))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
