//! The `BinaryExtensionInstance` module defines an instance to perform witness computations
//! for binary extension operations using the Binary Extension State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryExtensionSM` to compute witnesses for
//! execution plans.

use crate::{BinaryExtensionCollector, BinaryExtensionSM};
use data_bus::{BusDevice, PayloadType};
use p3_field::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use sm_common::{
    BusDeviceWrapper, CheckPoint, CollectSkipper, Instance, InstanceCtx, InstanceType,
};
use std::{collections::HashMap, sync::Arc};
use zisk_common::ChunkId;
use zisk_pil::BinaryExtensionTrace;

/// The `BinaryExtensionInstance` struct represents an instance for binary extension-related witness
/// computations.
///
/// It encapsulates the `BinaryExtensionSM` and its associated context, and it processes input data
/// to compute witnesses for binary extension operations.
pub struct BinaryExtensionInstance<F: PrimeField64> {
    /// Binary Extension state machine.
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl<F: PrimeField64> BinaryExtensionInstance<F> {
    /// Creates a new `BinaryExtensionInstance`.
    ///
    /// # Arguments
    /// * `binary_extension_sm` - An `Arc`-wrapped reference to the Binary Extension State Machine.
    /// * `instance_context` - The `InstanceCtx` associated with this instance, containing the
    ///   execution plan.
    ///
    /// # Returns
    /// A new `BinaryExtensionInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(binary_extension_sm: Arc<BinaryExtensionSM<F>>, ictx: InstanceCtx) -> Self {
        Self { binary_extension_sm, ictx }
    }
}

impl<F: PrimeField64> Instance<F> for BinaryExtensionInstance<F> {
    /// Computes the witness for the binary extension execution plan.
    ///
    /// This method leverages the `BinaryExtensionSM` to generate an `AirInstance` using the
    /// collected inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    /// * `_sctx` - The setup context, unused in this implementation.
    /// * `collectors` - A vector of input collectors to process and collect data for witness
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &mut self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, BusDeviceWrapper<PayloadType>)>,
    ) -> Option<AirInstance<F>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, mut collector)| {
                collector
                    .detach_device()
                    .as_any()
                    .downcast::<BinaryExtensionCollector>()
                    .unwrap()
                    .inputs
            })
            .collect();

        Some(self.binary_extension_sm.compute_witness(&inputs))
    }

    /// Retrieves the checkpoint associated with this instance.
    ///
    /// # Returns
    /// A `CheckPoint` object representing the checkpoint of the execution plan.
    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    /// Retrieves the type of this instance.
    ///
    /// # Returns
    /// An `InstanceType` representing the type of this instance (`InstanceType::Instance`).
    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            BinaryExtensionTrace::<F>::AIR_ID,
            "BinaryExtensionInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let (num_ops, collect_skipper) = collect_info[&chunk_id];
        Some(Box::new(BinaryExtensionCollector::new(num_ops as usize, collect_skipper)))
    }
}
