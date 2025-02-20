//! The `ArithFullInstance` module defines an instance to perform witness computations
//! for arithmetic-related operations using the Arithmetic Full State Machine.
//!
//! It manages collected inputs and interacts with the `ArithFullSM` to compute witnesses for
//! execution plans.

use crate::ArithFullSM;
use data_bus::{BusDevice, OperationData, PayloadType, OPERATION_BUS_ID};
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use sm_common::{
    input_collector, BusDeviceWrapper, CheckPoint, ChunkId, CollectSkipper, Instance, InstanceCtx,
    InstanceType,
};
use std::{collections::HashMap, sync::Arc};
use zisk_core::ZiskOperationType;
use zisk_pil::ArithTrace;

input_collector!(ArithInstanceCollector, ZiskOperationType::Arith, OperationData, OPERATION_BUS_ID);

/// The `ArithFullInstance` struct represents an instance for arithmetic-related witness
/// computations.
///
/// It encapsulates the `ArithFullSM` and its associated context, and it processes input data
/// to compute the witnesses for the arithmetic operations.
pub struct ArithFullInstance {
    /// Reference to the Arithmetic Full State Machine.
    arith_full_sm: Arc<ArithFullSM>,

    /// The instance context.
    ictx: InstanceCtx,
}

impl ArithFullInstance {
    /// Creates a new `ArithFullInstance`.
    ///
    /// # Arguments
    /// * `arith_full_sm` - An `Arc`-wrapped reference to the Arithmetic Full State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    ///
    /// # Returns
    /// A new `ArithFullInstance` instance initialized with the provided state machine and context.
    pub fn new(arith_full_sm: Arc<ArithFullSM>, ictx: InstanceCtx) -> Self {
        Self { arith_full_sm, ictx }
    }
}

impl<F: PrimeField> Instance<F> for ArithFullInstance {
    /// Computes the witness for the arithmetic execution plan.
    ///
    /// This method leverages the `ArithFullSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `pctx` - The proof context, unused in this implementation.
    /// * `sctx` - The setup context, unused in this implementation.
    /// * `collectors` - A vector of input collectors to process and collect data for witness
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &mut self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, mut collector)| {
                collector
                    .detach_device()
                    .as_any()
                    .downcast::<ArithInstanceCollector>()
                    .unwrap()
                    .inputs
            })
            .collect();

        Some(self.arith_full_sm.compute_witness(&inputs))
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
    fn build_inputs_collector(&self, chunk_id: usize) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            ArithTrace::<F>::AIR_ID,
            "BinaryInstance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let (num_ops, collect_skipper) = collect_info[&chunk_id];
        Some(Box::new(ArithInstanceCollector::new(num_ops, collect_skipper)))
    }
}
