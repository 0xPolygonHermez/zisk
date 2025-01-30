//! The `BinaryBasicInstance` module defines an instance to perform witness computations
//! for binary-related operations using the Binary Basic State Machine.
//!
//! It manages collected inputs and interacts with the `BinaryBasicSM` to compute witnesses for
//! execution plans.

use crate::BinaryBasicSM;
use data_bus::{BusDevice, BusId, OperationBusData, OperationData, PayloadType, OPERATION_BUS_ID};
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx};
use sm_common::{BusDeviceWrapper, CheckPoint, CollectSkipper, Instance, InstanceCtx, InstanceType};
use std::{any::Any, sync::Arc};
use zisk_core::ZiskOperationType;
use zisk_pil::BinaryTrace;

/// The `BinaryBasicInstance` struct represents an instance for binary-related witness computations.
///
/// It encapsulates the `BinaryBasicSM` and its associated context, and it processes input data
/// to compute witnesses for binary operations.
pub struct BinaryBasicInstance {
    /// Binary Basic state machine.
    binary_basic_sm: Arc<BinaryBasicSM>,

    /// Instance context.
    ictx: InstanceCtx,

    /// Collected inputs for witness computation.
    inputs: Vec<OperationData<u64>>,

    /// The connected bus ID.
    bus_id: BusId,
}

impl BinaryBasicInstance {
    /// Creates a new `BinaryBasicInstance`.
    ///
    /// # Arguments
    /// * `binary_basic_sm` - An `Arc`-wrapped reference to the Binary Basic State Machine.
    /// * `ictx` - The `InstanceCtx` associated with this instance, containing the execution plan.
    ///
    /// # Returns
    /// A new `BinaryBasicInstance` instance initialized with the provided state machine and
    /// context.
    pub fn new(binary_basic_sm: Arc<BinaryBasicSM>, ictx: InstanceCtx, bus_id: BusId) -> Self {
        Self { binary_basic_sm, ictx, inputs: Vec::new(), bus_id }
    }
}

impl<F: PrimeField> Instance<F> for BinaryBasicInstance {
    /// Computes the witness for the binary execution plan.
    ///
    /// This method leverages the `BinaryBasicSM` to generate an `AirInstance` using the collected
    /// inputs.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(&mut self, _pctx: &ProofCtx<F>) -> Option<AirInstance<F>> {
        Some(self.binary_basic_sm.compute_witness(&self.inputs))
    }

    fn compute_witness2(
        &mut self,
        _pctx: &ProofCtx<F>,
        collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let new_collectors = Vec::new();
        for collector in collectors {
            let (chunk_id, mut collector) = collector;
            let collector = collector.detach_device();
            let collector = collector.as_any().downcast::<BinaryBasicCollector>().unwrap();

            new_collectors.push(collector);
        }

        None
        // Some(self.binary_basic_sm.compute_witness2(collectors))
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
            id if id == BinaryTrace::<usize>::AIR_ID => {
                Some(Box::new(match &self.ictx.plan.check_point {
                    CheckPoint::Multiple2(check_point) => {
                        // check_point is an array
                        BinaryBasicCollector::new(OPERATION_BUS_ID, check_point[&chunk_id].1)
                    }
                    _ => panic!("Binary Basic: Invalid checkpoint type"),
                }))
            }
            _ => None,
        }
    }
}

impl BusDevice<u64> for BinaryBasicInstance {
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
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Binary as u32 {
            return (false, vec![]);
        }

        // if self.collect_skipper.should_skip() {
        //     return (false, vec![]);
        // }

        self.inputs.push(data);

        (self.inputs.len() == BinaryTrace::<usize>::NUM_ROWS, vec![])
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
pub struct BinaryBasicCollector {
    /// Collected inputs for witness computation.
    inputs: Vec<OperationData<u64>>,

    /// The connected bus ID.
    bus_id: BusId,

    /// Helper to skip instructions based on the plan's configuration.
    collect_skipper: CollectSkipper,
}

impl BinaryBasicCollector {
    pub fn new(bus_id: BusId, collect_skipper: CollectSkipper) -> Self {
        Self { inputs: Vec::new(), bus_id, collect_skipper }
    }
}

impl BusDevice<u64> for BinaryBasicCollector {
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
    fn process_data(&mut self, _bus_id: &BusId, data: &[u64]) -> (bool, Vec<(BusId, Vec<u64>)>) {
        let data: OperationData<u64> =
            data.try_into().expect("Regular Metrics: Failed to convert data");
        let op_type = OperationBusData::get_op_type(&data);

        if op_type as u32 != ZiskOperationType::Binary as u32 {
            return (false, vec![]);
        }

        if self.collect_skipper.should_skip() {
            return (false, vec![]);
        }

        self.inputs.push(data);

        (self.inputs.len() == BinaryTrace::<usize>::NUM_ROWS, vec![])
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
