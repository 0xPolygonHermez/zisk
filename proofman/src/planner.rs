use std::sync::Arc;

use common::ExecutionCtx;
use wchelpers::WCComponent;

pub trait Planner<F> {
    fn calculate_plan(&self, components: &[Arc<dyn WCComponent<F>>], ectx: &mut ExecutionCtx);
}

pub struct DefaultPlanner;

impl<F> Planner<F> for DefaultPlanner {
    fn calculate_plan(&self, components: &[Arc<dyn WCComponent<F>>], ectx: &mut ExecutionCtx) {
        for component in components.iter() {
            component.suggest_plan(ectx);
        }

        ectx.owned_instances = (0..ectx.instances.len()).collect();
    }
}
