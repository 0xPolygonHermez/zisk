use proofman_common::WitnessPilout;
use sm_common::LayoutPlanner;
use zisk_core::{EmuInstructionObserver, InstContext, ZiskInst};

/// A registry to manage all planners
#[derive(Default)]
pub struct PlannerRegistry {
    planners: Vec<Box<dyn LayoutPlanner>>,
}

impl PlannerRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self { planners: Vec::new() }
    }

    /// Register a new planner
    pub fn register_planner(&mut self, planner: Box<dyn LayoutPlanner>) {
        self.planners.push(planner);
    }

    pub fn new_session(&mut self, pilout: &WitnessPilout) {
        for planner in &mut self.planners {
            planner.new_session(pilout);
        }
    }

    /// Get the plans from all planners
    pub fn get_plans(&self) -> Vec<Box<dyn sm_common::OutputPlan>> {
        self.planners.iter().map(|planner| planner.get_plan()).collect()
    }
}

impl EmuInstructionObserver for PlannerRegistry {
    #[inline(always)]
    fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) {
        for planner in &mut self.planners {
            (*planner).on_instruction(zisk_inst, inst_ctx);
        }
    }
}
