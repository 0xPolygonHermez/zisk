//! The `ArithSM` module implements the Arithmetic State Machine,
//! coordinating sub-state machines to handle various arithmetic operations seamlessly.
//!
//! Key components of this module include:
//! - The `ArithSM` struct, encapsulating the full, table, and range table state machines.
//! - `ComponentBuilder` trait implementations for creating counters, planners, input collectors,
//!   and input generators specific to arithmetic computations.

use std::sync::Arc;

use fields::PrimeField64;
use zisk_common::{
    table_instance, BusDevice, BusDeviceMetrics, BusDeviceMode, ComponentBuilder, Instance,
    InstanceCtx, InstanceInfo, PayloadType, Planner, TableInfo, OPERATION_BUS_ID,
};
use zisk_core::ZiskOperationType;
use zisk_pil::{ArithRangeTableTrace, ArithTableTrace, ArithTrace};

use crate::{
    ArithCounterInputGen, ArithFullInstance, ArithFullSM, ArithPlanner, ArithRangeTableSM,
    ArithTableSM,
};

/// The `ArithSM` struct represents the Arithmetic State Machine, which
/// is a proxy machine to manage state machines involved in arithmetic operations.
pub struct ArithSM {
    /// Arith Full state machine
    arith_full_sm: Arc<ArithFullSM>,

    /// Arith Table state machine
    arith_table_sm: Arc<ArithTableSM>,

    /// Arith Range Table state machine
    arith_range_table_sm: Arc<ArithRangeTableSM>,
}

impl ArithSM {
    /// Creates a new instance of the `ArithSM` state machine.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithSM` containing initialized sub-state machines.
    pub fn new() -> Arc<Self> {
        let arith_table_sm = ArithTableSM::new();
        let arith_range_table_sm = ArithRangeTableSM::new();

        let arith_full_sm = ArithFullSM::new(arith_table_sm.clone(), arith_range_table_sm.clone());

        Arc::new(Self { arith_full_sm, arith_table_sm, arith_range_table_sm })
    }

    pub fn build_arith_counter(&self) -> ArithCounterInputGen {
        ArithCounterInputGen::new(BusDeviceMode::Counter)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for ArithSM {
    /// Builds and returns a new counter for monitoring arithmetic operations.
    ///
    /// # Returns
    /// A boxed implementation of `ArithCounter`.
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        Some(Box::new(ArithCounterInputGen::new(BusDeviceMode::Counter)))
    }

    /// Builds a planner to plan arithmetic-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `ArithPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(
            ArithPlanner::new()
                .add_instance(InstanceInfo::new(
                    ArithTrace::<usize>::AIRGROUP_ID,
                    ArithTrace::<usize>::AIR_ID,
                    ArithTrace::<usize>::NUM_ROWS,
                    ZiskOperationType::Arith,
                ))
                .add_table_instance(TableInfo::new(
                    ArithTableTrace::<usize>::AIRGROUP_ID,
                    ArithTableTrace::<usize>::AIR_ID,
                ))
                .add_table_instance(TableInfo::new(
                    ArithRangeTableTrace::<usize>::AIRGROUP_ID,
                    ArithRangeTableTrace::<usize>::AIR_ID,
                )),
        )
    }

    /// Builds an instance of the Arithmetic state machine.
    ///
    /// # Arguments
    /// * `ictx` - The context of the instance, containing the plan and its associated
    ///
    /// # Returns
    /// A boxed implementation of `StdInstance`.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            ArithTrace::<usize>::AIR_ID => {
                Box::new(ArithFullInstance::new(self.arith_full_sm.clone(), ictx))
            }
            ArithTableTrace::<usize>::AIR_ID => {
                table_instance!(ArithTableInstance, ArithTableSM, ArithTableTrace);
                Box::new(ArithTableInstance::new(
                    self.arith_table_sm.clone(),
                    ictx,
                    OPERATION_BUS_ID,
                ))
            }
            ArithRangeTableTrace::<usize>::AIR_ID => {
                table_instance!(ArithRangeTableInstance, ArithRangeTableSM, ArithRangeTableTrace);
                Box::new(ArithRangeTableInstance::new(
                    self.arith_range_table_sm.clone(),
                    ictx,
                    OPERATION_BUS_ID,
                ))
            }
            _ => panic!("BinarySM::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id),
        }
    }

    /// Creates and returns an input generator for arithmetic state machine computations.
    ///
    /// # Returns
    /// A boxed implementation of `ArithInputGenerator`.
    fn build_inputs_generator(&self) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(ArithCounterInputGen::new(BusDeviceMode::InputGenerator)))
    }
}
