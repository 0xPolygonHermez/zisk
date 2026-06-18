use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;

use zisk_common::{BusDeviceMode, ComponentBuilder, Instance, InstanceCtx, InstanceInfo, Planner};
use zisk_core::ZiskOperationType;
use zisk_pil::BabyJubJubTrace;

use crate::{BabyJubJubCounterInputGen, BabyJubJubInstance, BabyJubJubPlanner, BabyJubJubSM};

/// The `BabyJubJubManager` struct manages the BabyJubJub state machine.
#[allow(dead_code)]
pub struct BabyJubJubManager<F: PrimeField64> {
    /// BabyJubJub state machine.
    babyjubjub_sm: Arc<BabyJubJubSM<F>>,
}

impl<F: PrimeField64> BabyJubJubManager<F> {
    /// Creates a new instance of `BabyJubJubManager`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let babyjubjub_sm = BabyJubJubSM::new(std);

        Arc::new(Self { babyjubjub_sm })
    }

    pub fn build_babyjubjub_counter(&self, asm_execution: bool) -> BabyJubJubCounterInputGen {
        match asm_execution {
            true => BabyJubJubCounterInputGen::new(BusDeviceMode::CounterAsm),
            false => BabyJubJubCounterInputGen::new(BusDeviceMode::Counter),
        }
    }

    pub fn build_babyjubjub_input_generator(&self) -> BabyJubJubCounterInputGen {
        BabyJubJubCounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for BabyJubJubManager<F> {
    /// Builds a planner to plan BabyJubJub instances.
    fn build_planner(&self) -> Box<dyn Planner> {
        let num_available_ops = self.babyjubjub_sm.num_available_ops;

        Box::new(BabyJubJubPlanner::new().add_instance(InstanceInfo::new(
            BabyJubJubTrace::<()>::AIRGROUP_ID,
            BabyJubJubTrace::<()>::AIR_ID,
            num_available_ops,
            ZiskOperationType::BabyJubJub,
        )))
    }

    /// Builds an instance for the requested `air_id`.
    ///
    /// # Panics
    /// Panics if the provided `air_id` is not supported.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            id if id == BabyJubJubTrace::<()>::AIR_ID => {
                Box::new(BabyJubJubInstance::new(self.babyjubjub_sm.clone(), ictx))
            }
            _ => {
                panic!(
                    "BabyJubJubManager::build_instance() Unsupported air_id: {:?}",
                    ictx.plan.air_id
                )
            }
        }
    }
}
