use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use zisk_common::{
    BusDevice, BusDeviceMetrics, BusDeviceMode, ComponentBuilder, Instance, InstanceCtx,
    InstanceInfo, PayloadType, Planner,
};
use zisk_core::ZiskOperationType;
#[cfg(not(feature = "packed"))]
use zisk_pil::Add256Trace;
#[cfg(feature = "packed")]
use zisk_pil::Add256TracePacked;

#[cfg(not(feature = "packed"))]
type Add256TraceType<F> = Add256Trace<F>;
#[cfg(feature = "packed")]
type Add256TraceType<F> = Add256TracePacked<F>;

use crate::{Add256CounterInputGen, Add256Instance, Add256Planner, Add256SM};

/// The `Add256Manager` struct represents the Add256 manager,
/// which is responsible for managing the Add256 state machine and its table state machine.
#[allow(dead_code)]
pub struct Add256Manager<F: PrimeField64> {
    /// Add256 state machine
    add256_sm: Arc<Add256SM<F>>,
}

impl<F: PrimeField64> Add256Manager<F> {
    /// Creates a new instance of `Add256Manager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `Add256Manager`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let add256_sm = Add256SM::new(std);

        Arc::new(Self { add256_sm })
    }

    pub fn build_add256_counter(&self) -> Add256CounterInputGen {
        Add256CounterInputGen::new(BusDeviceMode::Counter)
    }

    pub fn build_add256_input_generator(&self) -> Add256CounterInputGen {
        Add256CounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for Add256Manager<F> {
    /// Builds and returns a new counter for monitoring Add256 operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for Add256 operations.
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        Some(Box::new(Add256CounterInputGen::new(BusDeviceMode::Counter)))
    }

    /// Builds a planner to plan Add256-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        // Get the number of Add256s that a single Add256 instance can handle
        let num_availables = self.add256_sm.num_availables;

        Box::new(Add256Planner::new().add_instance(InstanceInfo::new(
            Add256TraceType::<F>::AIRGROUP_ID,
            Add256TraceType::<F>::AIR_ID,
            num_availables,
            ZiskOperationType::BigInt,
        )))
    }

    /// Builds an inputs data collector for Add256 operations.
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
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            id if id == Add256TraceType::<F>::AIR_ID => {
                Box::new(Add256Instance::new(self.add256_sm.clone(), ictx))
            }
            _ => {
                panic!("Add256Builder::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id)
            }
        }
    }

    fn build_inputs_generator(&self) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(Add256CounterInputGen::new(BusDeviceMode::InputGenerator)))
    }
}
