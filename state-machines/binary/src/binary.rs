//! The `BinarySM` module implements the Binary State Machine,
//! coordinating sub-state machines to handle various binary operations seamlessly.
//!
//! Key components of this module include:
//! - The `BinarySM` struct, encapsulating the basic and extension state machines along with their
//!   table counterparts.
//! - `ComponentBuilder` trait implementations for creating counters, planners, and input collectors
//!   specific to binary operations.

use std::sync::Arc;

use crate::{
    BinaryBasicInstanceBuilder, BinaryBasicSM, BinaryBasicTableSM, BinaryExtensionInstanceBuilder,
    BinaryExtensionSM, BinaryExtensionTableSM,
};
use p3_field::PrimeField;
use pil_std_lib::Std;
use sm_common::{
    table_instance, BusDeviceInstance, BusDeviceMetrics, ComponentBuilder, InstanceCtx,
    InstanceInfo, Planner, RegularCounters, RegularPlanner, TableInfo,
};
use zisk_common::OPERATION_BUS_ID;
use zisk_core::ZiskOperationType;
use zisk_pil::{BinaryExtensionTableTrace, BinaryExtensionTrace, BinaryTableTrace, BinaryTrace};

/// The `BinarySM` struct represents the Binary State Machine,
/// managing both basic and extension binary operations.
#[allow(dead_code)]
pub struct BinarySM<F: PrimeField> {
    /// PIL2 standard library
    std: Arc<Std<F>>,

    /// Binary Basic state machine
    binary_basic_sm: Arc<BinaryBasicSM>,

    /// Binary Basic Table state machine
    binary_basic_table_sm: Arc<BinaryBasicTableSM>,

    /// Binary Extension state machine
    binary_extension_sm: Arc<BinaryExtensionSM>,

    /// Binary Extension Table state machine
    binary_extension_table_sm: Arc<BinaryExtensionTableSM>,
}

impl<F: PrimeField> BinarySM<F> {
    /// Creates a new instance of the `BinarySM` state machine.
    ///
    /// # Arguments
    /// * `std` - PIL2 standard library utilities.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `BinarySM`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let binary_basic_table_sm = BinaryBasicTableSM::new::<F>();
        let binary_basic_sm = BinaryBasicSM::new();

        let binary_extension_table_sm = BinaryExtensionTableSM::new::<F>();
        let binary_extension_sm = BinaryExtensionSM::new();

        Arc::new(Self {
            std,
            binary_basic_sm,
            binary_basic_table_sm,
            binary_extension_sm,
            binary_extension_table_sm,
        })
    }
}

impl<F: PrimeField> ComponentBuilder<F> for BinarySM<F> {
    /// Builds and returns a new counter for monitoring binary operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for binary and extension binary
    /// operations.
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(RegularCounters::new(
            OPERATION_BUS_ID,
            vec![ZiskOperationType::Binary, ZiskOperationType::BinaryE],
        ))
    }

    /// Builds a planner to plan binary-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(
            RegularPlanner::new()
                .add_instance(InstanceInfo::new(
                    BinaryTrace::<usize>::AIR_ID,
                    BinaryTrace::<usize>::AIRGROUP_ID,
                    BinaryTrace::<usize>::NUM_ROWS,
                    ZiskOperationType::Binary,
                ))
                .add_instance(InstanceInfo::new(
                    BinaryExtensionTrace::<usize>::AIR_ID,
                    BinaryExtensionTrace::<usize>::AIRGROUP_ID,
                    BinaryExtensionTrace::<usize>::NUM_ROWS,
                    ZiskOperationType::BinaryE,
                ))
                .add_table_instance(TableInfo::new(
                    BinaryTableTrace::<usize>::AIR_ID,
                    BinaryTableTrace::<usize>::AIRGROUP_ID,
                ))
                .add_table_instance(TableInfo::new(
                    BinaryExtensionTableTrace::<usize>::AIR_ID,
                    BinaryExtensionTableTrace::<usize>::AIRGROUP_ID,
                )),
        )
    }

    /// Builds an inputs data collector for binary operations.
    ///
    /// # Arguments
    /// * `ictx` - The context of the instance, containing the plan and its associated
    ///   configurations.
    ///
    /// # Returns
    /// A boxed implementation of `BusDeviceInstance` specific to the requested `air_id` instance.
    ///
    /// # Panics
    /// Panics if the provided `air_id` is not supported.
    fn build_inputs_collector(&self, ictx: InstanceCtx) -> Box<dyn BusDeviceInstance<F>> {
        match ictx.plan.air_id {
            id if id == BinaryTrace::<usize>::AIR_ID => BinaryBasicInstanceBuilder::build(
                self.binary_basic_table_sm.clone(),
                ictx,
                OPERATION_BUS_ID,
            ),
            id if id == BinaryExtensionTrace::<usize>::AIR_ID => {
                BinaryExtensionInstanceBuilder::build(
                    self.std.clone(),
                    self.binary_extension_table_sm.clone(),
                    ictx,
                    OPERATION_BUS_ID,
                )
            }
            id if id == BinaryTableTrace::<usize>::AIR_ID => {
                table_instance!(BinaryBasicTableInstance, BinaryBasicTableSM, BinaryTableTrace);
                Box::new(BinaryBasicTableInstance::new(
                    self.binary_basic_table_sm.clone(),
                    ictx,
                    OPERATION_BUS_ID,
                ))
            }
            id if id == BinaryExtensionTableTrace::<usize>::AIR_ID => {
                table_instance!(
                    BinaryExtensionTableInstance,
                    BinaryExtensionTableSM,
                    BinaryExtensionTableTrace
                );
                Box::new(BinaryExtensionTableInstance::new(
                    self.binary_extension_table_sm.clone(),
                    ictx,
                    OPERATION_BUS_ID,
                ))
            }
            _ => panic!("BinarySM::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id),
        }
    }
}
