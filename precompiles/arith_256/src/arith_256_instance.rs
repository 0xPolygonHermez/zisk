//! The `Arith256Instance` module defines an instance to perform the witness computation
//! for the Arith256 State Machine.
//!
//! It manages collected inputs and interacts with the `Arith256SM` to compute witnesses for
//! execution plans.

use crate::Arith256SM;
use data_bus::{
    BusDevice, BusId, ExtOperationData, OperationArith256Data, OperationBusData, PayloadType,
    OPERATION_BUS_ID,
};
use p3_field::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use sm_common::{
    BusDeviceWrapper, CheckPoint, ChunkId, CollectSkipper, Instance, InstanceCtx, InstanceType,
};
use std::{any::Any, collections::HashMap, sync::Arc};
use zisk_core::ZiskOperationType;
use zisk_pil::Arith256Trace;

/// The `Arith256Instance` struct represents an instance for the Arith256 State Machine.
///
/// It encapsulates the `Arith256SM` and its associated context, and it processes input data
/// to compute witnesses for the Arith256 State Machine.
pub struct Arith256Instance {
    /// Arith256 state machine.
    arith256_sm: Arc<Arith256SM>,

    /// Instance context.
    ictx: InstanceCtx,
}

impl Arith256Instance {
    /// Creates a new `Arith256Instance`.
    ///
    /// # Arguments
    /// * `arith256_sm` - An `Arc`-wrapped reference to the Arith256 State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    /// * `bus_id` - The bus ID associated with this instance.
    ///
    /// # Returns
    /// A new `Arith256Instance` instance initialized with the provided state machine and
    /// context.
    pub fn new(arith256_sm: Arc<Arith256SM>, ictx: InstanceCtx) -> Self {
        Self { arith256_sm, ictx }
    }
}

impl<F: PrimeField64> Instance<F> for Arith256Instance {
    /// Computes the witness for the arith256 execution plan.
    ///
    /// This method leverages the `Arith256SM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &mut self,
        _pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, mut collector)| {
                collector.detach_device().as_any().downcast::<Arith256Collector>().unwrap().inputs
            })
            .collect();

        Some(self.arith256_sm.compute_witness(sctx, &inputs))
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

    fn build_inputs_collector(&self, chunk_id: usize) -> Option<Box<dyn BusDevice<PayloadType>>> {
        assert_eq!(
            self.ictx.plan.air_id,
            Arith256Trace::<F>::AIR_ID,
            "Arith256Instance: Unsupported air_id: {:?}",
            self.ictx.plan.air_id
        );

        let meta = self.ictx.plan.meta.as_ref().unwrap();
        let collect_info = meta.downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let (num_ops, collect_skipper) = collect_info[&chunk_id];
        Some(Box::new(Arith256Collector::new(num_ops, collect_skipper)))
    }
}

pub struct Arith256Collector {
    /// Collected inputs for witness computation.
    inputs: Vec<OperationArith256Data<u64>>,

    /// The number of operations to collect.
    num_operations: u64,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl Arith256Collector {
    /// Creates a new `Arith256Collector`.
    ///
    /// # Arguments
    ///
    /// * `bus_id` - The connected bus ID.
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_skipper` - The helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `ArithInstanceCollector` instance initialized with the provided parameters.
    pub fn new(num_operations: u64, collect_skipper: CollectSkipper) -> Self {
        Self { inputs: Vec::new(), num_operations, collect_skipper }
    }
}

impl BusDevice<PayloadType> for Arith256Collector {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A tuple where:
    /// - The first element indicates whether further processing should continue.
    /// - The second element contains derived inputs to be sent back to the bus (always empty).
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
    ) -> Option<Vec<(BusId, Vec<PayloadType>)>> {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return None;
        }

        let data: ExtOperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");

        if OperationBusData::get_op_type(&data) as u32 != ZiskOperationType::Arith256 as u32 {
            return None;
        }

        if self.collect_skipper.should_skip() {
            return None;
        }

        if let ExtOperationData::OperationArith256Data(data) = data {
            self.inputs.push(data);
            None
        } else {
            panic!("Expected ExtOperationData::OperationData");
        }
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![OPERATION_BUS_ID]
    }

    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
