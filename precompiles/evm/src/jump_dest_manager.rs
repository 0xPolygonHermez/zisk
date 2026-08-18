//! Ties the `jump_dest` pieces together for the executor.
//!
//! One AIR, one state machine, so this is the whole of the DMA manager's job
//! with the branching removed: hand out the counter and the planner during the
//! count-and-plan phase, and build an instance per planned segment afterwards.

use std::sync::Arc;

use pil2_std_lib::Std;
use proofman_common::ProofCtx;
use proofman_fields::PrimeField64;
use zisk_common::{
    BusDeviceMode, ComponentBuilder, ComponentPlanBuilder, Instance, InstanceCtx, Plan, Planner,
};
use zisk_pil::{JumpDestTrace, ZiskProofValues};

use crate::{JumpDestCounterInputGen, JumpDestInstance, JumpDestPlanner, JumpDestSM};

pub struct JumpDestManager<F: PrimeField64> {
    jump_dest_sm: Arc<JumpDestSM<F>>,
}

impl<F: PrimeField64> JumpDestManager<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { jump_dest_sm: JumpDestSM::new(std) })
    }
}

impl<F: PrimeField64> ComponentPlanBuilder<F> for JumpDestManager<F> {
    type Counter = JumpDestCounterInputGen;

    fn counter(is_asm_emulator: bool) -> Self::Counter {
        let mode = if is_asm_emulator { BusDeviceMode::CounterAsm } else { BusDeviceMode::Counter };
        JumpDestCounterInputGen::new(mode)
    }

    fn planner(_is_asm_emulator: bool) -> Box<dyn Planner> {
        Box::new(JumpDestPlanner::<F>::new())
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for JumpDestManager<F> {
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            JumpDestTrace::<()>::AIR_ID => {
                Box::new(JumpDestInstance::new(self.jump_dest_sm.clone(), ictx))
            }
            _ => panic!(
                "JumpDestManager::build_instance() unsupported air_id: {:?}",
                ictx.plan.air_id
            ),
        }
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &[Plan]) {
        let enable = plannings.iter().any(|p| p.air_id == JumpDestTrace::<()>::AIR_ID);
        let mut proof_values = ZiskProofValues::from_vec_guard(pctx.get_proof_values());
        proof_values.enable_jump_dest = F::from_bool(enable);
    }
}
