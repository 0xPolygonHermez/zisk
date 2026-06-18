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
    BinaryAddInstance, BinaryAddSM, BinaryBasicInstance, BinaryBasicSM, BinaryCounter,
    BinaryExtensionInstance, BinaryExtensionSM, BinaryPlanner,
};
use fields::PrimeField64;
use zisk_common::{
    ComponentBuilder, ComponentPlanBuilder, Instance, InstanceCtx, Planner, RangeChecker,
};
use zisk_pil::{BinaryAddTrace, BinaryExtensionTrace, BinaryTrace};

/// The `BinarySM` struct represents the Binary State Machine,
/// managing basic, extension and specific add binary operations.
#[allow(dead_code)]
pub struct BinarySM<F: PrimeField64, RC: RangeChecker> {
    /// Binary Basic state machine
    binary_basic_sm: Arc<BinaryBasicSM<F, RC>>,

    /// Binary Extension state machine
    binary_extension_sm: Arc<BinaryExtensionSM<F, RC>>,

    /// Binary Add state machine (optimal only for addition)
    binary_add_sm: Arc<BinaryAddSM<F, RC>>,

    std: Arc<RC>,
}

impl<F: PrimeField64, RC: RangeChecker> BinarySM<F, RC> {
    /// Creates a new instance of the `BinarySM` state machine.
    ///
    /// # Arguments
    /// * `std` - the range-check / virtual-table sink.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `BinarySM`.
    pub fn new(std: Arc<RC>) -> Arc<Self> {
        let binary_basic_sm = BinaryBasicSM::new(std.clone());

        let binary_extension_sm = BinaryExtensionSM::new(std.clone());

        let binary_add_sm = BinaryAddSM::new(std.clone());

        Arc::new(Self { binary_basic_sm, binary_extension_sm, binary_add_sm, std })
    }
}

impl<F: PrimeField64, RC: RangeChecker> ComponentPlanBuilder<F> for BinarySM<F, RC> {
    type Counter = BinaryCounter;

    fn counter(_is_asm_emulator: bool) -> Self::Counter {
        BinaryCounter::new()
    }

    fn planner(_is_asm_emulator: bool) -> Box<dyn Planner> {
        Box::new(BinaryPlanner::<F>::new())
    }
}

impl<F: PrimeField64, RC: RangeChecker> ComponentBuilder<F> for BinarySM<F, RC> {
    /// Builds an instance for binary operations.
    ///
    /// # Arguments
    /// * `ictx` - The instance context.
    ///
    /// # Returns
    /// A boxed implementation of `Instance` for binary operations.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            BinaryTrace::<()>::AIR_ID => Box::new(BinaryBasicInstance::new(
                self.binary_basic_sm.clone(),
                ictx,
                self.std.clone(),
            )),
            BinaryExtensionTrace::<()>::AIR_ID => Box::new(BinaryExtensionInstance::new(
                self.binary_extension_sm.clone(),
                ictx,
                self.std.clone(),
            )),
            BinaryAddTrace::<()>::AIR_ID => {
                Box::new(BinaryAddInstance::new(self.binary_add_sm.clone(), ictx, self.std.clone()))
            }
            _ => panic!("BinarySM::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id),
        }
    }
}
