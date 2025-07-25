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
    BinaryAddInstance, BinaryAddSM, BinaryBasicInstance, BinaryBasicSM, BinaryBasicTableSM,
    BinaryCounter, BinaryExtensionInstance, BinaryExtensionSM, BinaryExtensionTableSM,
    BinaryPlanner,
};
use fields::PrimeField64;
use pil_std_lib::Std;
use zisk_common::{
    table_instance, BusDeviceMetrics, ComponentBuilder, Instance, InstanceCtx, Planner,
    OPERATION_BUS_ID,
};
use zisk_pil::{
    BinaryAddTrace, BinaryExtensionTableTrace, BinaryExtensionTrace, BinaryTableTrace, BinaryTrace,
};

/// The `BinarySM` struct represents the Binary State Machine,
/// managing basic, extension and specific add binary operations.
#[allow(dead_code)]
pub struct BinarySM<F: PrimeField64> {
    /// Binary Basic state machine
    binary_basic_sm: Arc<BinaryBasicSM>,

    /// Binary Basic Table state machine
    binary_basic_table_sm: Arc<BinaryBasicTableSM>,

    /// Binary Extension state machine
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,

    /// Binary Extension Table state machine
    binary_extension_table_sm: Arc<BinaryExtensionTableSM>,

    /// Binary Add state machine (optimal only for addition)
    binary_add_sm: Arc<BinaryAddSM<F>>,
}

impl<F: PrimeField64> BinarySM<F> {
    /// Creates a new instance of the `BinarySM` state machine.
    ///
    /// # Arguments
    /// * `std` - PIL2 standard library utilities.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `BinarySM`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let binary_basic_table_sm = BinaryBasicTableSM::new();
        let binary_basic_sm = BinaryBasicSM::new(binary_basic_table_sm.clone());

        let binary_extension_table_sm = BinaryExtensionTableSM::new();
        let binary_extension_sm =
            BinaryExtensionSM::new(std.clone(), binary_extension_table_sm.clone());

        let binary_add_sm = BinaryAddSM::new(std);

        Arc::new(Self {
            binary_basic_sm,
            binary_basic_table_sm,
            binary_extension_sm,
            binary_extension_table_sm,
            binary_add_sm,
        })
    }

    pub fn build_binary_counter(&self) -> BinaryCounter {
        BinaryCounter::new()
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for BinarySM<F> {
    /// Builds and returns a new counter for monitoring binary operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for binary and extension binary
    /// operations.
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        Some(Box::new(BinaryCounter::new()))
    }

    /// Builds a planner to plan binary-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(BinaryPlanner::new())
    }

    /// Builds an instance for binary operations.
    ///
    /// # Arguments
    /// * `ictx` - The instance context.
    ///
    /// # Returns
    /// A boxed implementation of `Instance` for binary operations.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            BinaryTrace::<usize>::AIR_ID => {
                Box::new(BinaryBasicInstance::new(self.binary_basic_sm.clone(), ictx))
            }
            BinaryAddTrace::<usize>::AIR_ID => {
                Box::new(BinaryAddInstance::new(self.binary_add_sm.clone(), ictx))
            }
            BinaryExtensionTrace::<usize>::AIR_ID => {
                Box::new(BinaryExtensionInstance::new(self.binary_extension_sm.clone(), ictx))
            }
            BinaryTableTrace::<usize>::AIR_ID => {
                table_instance!(BinaryBasicTableInstance, BinaryBasicTableSM, BinaryTableTrace);

                Box::new(BinaryBasicTableInstance::new(
                    self.binary_basic_table_sm.clone(),
                    ictx,
                    OPERATION_BUS_ID,
                ))
            }
            BinaryExtensionTableTrace::<usize>::AIR_ID => {
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
