use std::{path::PathBuf, sync::Arc};

use data_bus::{BusDevice, PayloadType, OPERATION_BUS_ID};
use p3_field::PrimeField64;

use sm_common::{
    table_instance_array, BusDeviceMetrics, BusDeviceMode, ComponentBuilder, Instance, InstanceCtx,
    InstanceInfo, Planner, TableInfo,
};
use zisk_core::ZiskOperationType;
use zisk_pil::{Sha256fTableTrace, Sha256fTrace};

use crate::{Sha256fCounterInputGen, Sha256fInstance, Sha256fPlanner, Sha256fSM, Sha256fTableSM};

/// The `Sha256fManager` struct represents the Sha256f manager,
/// which is responsible for managing the Sha256f state machine and its table state machine.
#[allow(dead_code)]
pub struct Sha256fManager {
    /// Sha256f state machine
    sha256f_sm: Arc<Sha256fSM>,

    /// Sha256f table state machine
    sha256f_table_sm: Arc<Sha256fTableSM>,
}

impl Sha256fManager {
    /// Creates a new instance of `Sha256fManager`.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `Sha256fManager`.
    pub fn new<F: PrimeField64>(script_path: PathBuf) -> Arc<Self> {
        let sha256f_table_sm = Sha256fTableSM::new::<F>();
        let sha256f_sm = Sha256fSM::new(sha256f_table_sm.clone(), script_path);

        Arc::new(Self { sha256f_sm, sha256f_table_sm })
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for Sha256fManager {
    /// Builds and returns a new counter for monitoring sha256f operations.
    ///
    /// # Returns
    /// A boxed implementation of `RegularCounters` configured for sha256f operations.
    fn build_counter(&self) -> Box<dyn BusDeviceMetrics> {
        Box::new(Sha256fCounterInputGen::new(BusDeviceMode::Counter))
    }

    /// Builds a planner to plan sha256f-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RegularPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        // Get the number of sha256fs that a single sha256f instance can handle
        let num_available_sha256fs = self.sha256f_sm.num_available_sha256fs;

        Box::new(
            Sha256fPlanner::new()
                .add_instance(InstanceInfo::new(
                    Sha256fTrace::<usize>::AIRGROUP_ID,
                    Sha256fTrace::<usize>::AIR_ID,
                    num_available_sha256fs,
                    ZiskOperationType::Sha256,
                ))
                .add_table_instance(TableInfo::new(
                    Sha256fTableTrace::<usize>::AIRGROUP_ID,
                    Sha256fTableTrace::<usize>::AIR_ID,
                )),
        )
    }

    /// Builds an inputs data collector for sha256f operations.
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
            id if id == Sha256fTrace::<usize>::AIR_ID => {
                Box::new(Sha256fInstance::new(self.sha256f_sm.clone(), ictx))
            }
            id if id == Sha256fTableTrace::<usize>::AIR_ID => {
                table_instance_array!(Sha256fTableInstance, Sha256fTableSM, Sha256fTableTrace);
                Box::new(Sha256fTableInstance::new(
                    self.sha256f_table_sm.clone(),
                    ictx,
                    OPERATION_BUS_ID,
                ))
            }
            _ => {
                panic!("Sha256fBuilder::get_instance() Unsupported air_id: {:?}", ictx.plan.air_id)
            }
        }
    }

    fn build_inputs_generator(&self) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(Sha256fCounterInputGen::new(BusDeviceMode::InputGenerator)))
    }
}
