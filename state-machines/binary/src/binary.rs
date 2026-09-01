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
    BinaryAddHiInstance, BinaryAddHiSM, BinaryAddInstance, BinaryAddSM, BinaryBasicInstance,
    BinaryBasicSM, BinaryCounter, BinaryExtensionInstance, BinaryExtensionSM, BinaryPlanner,
};
use pil2_std_lib::Std;
use proofman_fields::PrimeField64;
use zisk_common::{ComponentBuilder, ComponentPlanBuilder, Instance, InstanceCtx, Planner};
use zisk_pil::{
    BinaryAddHiLargeTrace, BinaryAddHiTrace, BinaryAddLargeTrace, BinaryAddTrace,
    BinaryExtensionLargeTrace, BinaryExtensionTrace, BinaryLargeTrace, BinaryTrace,
};

/// The `BinarySM` struct represents the Binary State Machine,
/// managing basic, extension and specific add binary operations.
#[allow(dead_code)]
pub struct BinarySM<F: PrimeField64> {
    /// Binary Basic state machine
    binary_basic_sm: Arc<BinaryBasicSM<F>>,

    /// Binary Extension state machine
    binary_extension_sm: Arc<BinaryExtensionSM<F>>,

    /// Binary Add state machine (optimal only for addition)
    binary_add_sm: Arc<BinaryAddSM<F>>,

    /// Binary Add Hi state machine (packs the additions that fit in the low 32-bit limb)
    binary_add_hi_sm: Arc<BinaryAddHiSM<F>>,

    std: Arc<Std<F>>,
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
        let binary_basic_sm = BinaryBasicSM::new(std.clone());

        let binary_extension_sm = BinaryExtensionSM::new(std.clone());

        let binary_add_sm = BinaryAddSM::new(std.clone());

        let binary_add_hi_sm = BinaryAddHiSM::new(std.clone());

        Arc::new(Self {
            binary_basic_sm,
            binary_extension_sm,
            binary_add_sm,
            binary_add_hi_sm,
            std,
        })
    }
}

impl<F: PrimeField64> ComponentPlanBuilder<F> for BinarySM<F> {
    type Counter = BinaryCounter;

    fn counter(_is_asm_emulator: bool) -> Self::Counter {
        BinaryCounter::new()
    }

    fn planner(_is_asm_emulator: bool) -> Box<dyn Planner> {
        Box::new(BinaryPlanner::<F>::new())
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for BinarySM<F> {
    /// Builds an instance for binary operations.
    ///
    /// # Arguments
    /// * `ictx` - The instance context.
    ///
    /// # Returns
    /// A boxed implementation of `Instance` for binary operations.
    /// Each air and its `Large` sibling share one instance type, which picks the trace — and with it
    /// the height and air id — from `ictx.plan.air_id`.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            BinaryTrace::<()>::AIR_ID | BinaryLargeTrace::<()>::AIR_ID => Box::new(
                BinaryBasicInstance::new(self.binary_basic_sm.clone(), ictx, self.std.clone()),
            ),
            BinaryExtensionTrace::<()>::AIR_ID | BinaryExtensionLargeTrace::<()>::AIR_ID => {
                Box::new(BinaryExtensionInstance::new(
                    self.binary_extension_sm.clone(),
                    ictx,
                    self.std.clone(),
                ))
            }
            BinaryAddTrace::<()>::AIR_ID | BinaryAddLargeTrace::<()>::AIR_ID => {
                Box::new(BinaryAddInstance::new(self.binary_add_sm.clone(), ictx, self.std.clone()))
            }
            BinaryAddHiTrace::<()>::AIR_ID | BinaryAddHiLargeTrace::<()>::AIR_ID => Box::new(
                BinaryAddHiInstance::new(self.binary_add_hi_sm.clone(), ictx, self.std.clone()),
            ),
            _ => panic!("BinarySM::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id),
        }
    }
}
