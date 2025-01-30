//! The `BinaryExtensionInstance` module defines an instance to perform witness computations
//! for binary extension operations using the Binary Extension State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryExtensionSM` to compute witnesses for
//! execution plans.

use crate::BinaryExtensionSM;
use data_bus::{BusDevice, BusId, OperationBusData, OperationData, PayloadType, OPERATION_BUS_ID};
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{CheckPoint, CollectSkipper, Instance, InstanceCtx, InstanceType};
use std::sync::Arc;
use zisk_core::ZiskOperationType;
use zisk_pil::BinaryExtensionTrace;

/// The `BinaryExtensionInstance` struct represents an instance for binary extension-related witness
/// computations.
///
/// It encapsulates the `BinaryExtensionSM` and its associated context, and it processes input data
/// to compute witnesses for binary extension operations.
pub struct BinaryExtensionInstance<F: PrimeField> {
    /// Binary Extension state machine.
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,

    /// Instance context.
    ictx: InstanceCtx,

    /// Collected inputs for witness computation.
    inputs: Vec<OperationData<u64>>,

    /// The connected bus ID.
    bus_id: BusId,
}

impl<F: PrimeField> BinaryExtensionInstance<F> {
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
    pub fn new(
        binary_extension_sm: Arc<BinaryExtensionSM<F>>,
        ictx: InstanceCtx,
        bus_id: BusId,
    ) -> Self {
        Self { binary_extension_sm, ictx, inputs: Vec::new(), bus_id }
    }
}

impl<F: PrimeField> Instance<F> for BinaryExtensionInstance<F> {
    /// Computes the witness for the binary extension execution plan.
    ///
    /// This method leverages the `BinaryExtensionSM` to generate an `AirInstance` using the
    /// collected inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(self.binary_extension_sm.compute_witness(&self.inputs))
    }

    fn compute_witness2(
        &mut self,
        pctx: &ProofCtx<F>,
        collectors: Vec<(usize, Box<sm_common::BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let mut collectors2: Vec<Box<dyn BusDevice<u64>>> = Vec::new();

        // for collector in collectors {
        //     if collector.1.bus_device().as_any().is::<BinaryExtensionCollector>() {
        //         collectors2.push(collector.1); // Move the entire `Box<dyn BusDevice>`
        //     }
        // }

        None
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

    fn build_inputs_collector2(&self, chunk_id: usize) -> Option<Box<dyn BusDevice<PayloadType>>> {
        match self.ictx.plan.air_id {
            id if id == BinaryExtensionTrace::<usize>::AIR_ID => {
                Some(Box::new(match &self.ictx.plan.check_point {
                    CheckPoint::Multiple2(check_point) => {
                        BinaryExtensionCollector::new(OPERATION_BUS_ID, check_point[&chunk_id].1)
                    }
                    _ => panic!("Binary Extension: Invalid checkpoint type"),
                }))
            }
            _ => None,
        }
    }
}

impl<F: PrimeField> BusDevice<u64> for BinaryExtensionInstance<F> {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A tuple where:
    /// - The first element indicates whether further processing should continue.
    /// - The second element is always empty.
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::BinaryE as u32 {
            return (false, vec![]);
        }

        // if self.collect_skipper.should_skip() {
        //     return (false, vec![]);
        // }

        self.inputs.push(data);

        (self.inputs.len() == BinaryExtensionTrace::<usize>::NUM_ROWS, vec![])
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

///////////////////////////////////
pub struct BinaryExtensionCollector {
    /// Collected inputs for witness computation.
    inputs: Vec<OperationData<u64>>,

    /// The connected bus ID.
    bus_id: BusId,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl BinaryExtensionCollector {
    pub fn new(bus_id: BusId, collect_skipper: CollectSkipper) -> Self {
        Self { inputs: Vec::new(), bus_id, collect_skipper }
    }
}

impl BusDevice<u64> for BinaryExtensionCollector {
    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A tuple where:
    /// - The first element indicates whether further processing should continue.
    /// - The second element is always empty.
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::BinaryE as u32 {
            return (false, vec![]);
        }

        if self.collect_skipper.should_skip() {
            return (false, vec![]);
        }

        self.inputs.push(data);

        (self.inputs.len() == BinaryExtensionTrace::<usize>::NUM_ROWS, vec![])
    }

    /// Returns the bus IDs associated with this instance.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![self.bus_id]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
