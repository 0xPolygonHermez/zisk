use std::rc::Rc;

use common::ExecutionCtx;
use wchelpers::WCComponent;

pub trait Planner<F> {
    fn calculate_plan(&self, components: &[Rc<dyn WCComponent<F>>], ectx: &mut ExecutionCtx);
}

pub struct DefaultPlanner;

impl<F> Planner<F> for DefaultPlanner {
    fn calculate_plan(&self, components: &[Rc<dyn WCComponent<F>>], ectx: &mut ExecutionCtx) {
        let mut last_idx;
        for (component_idx, component) in components.iter().enumerate() {
            last_idx = ectx.instances.len();
            component.calculate_plan(ectx);
            for i in last_idx..ectx.instances.len() {
                ectx.instances[i].wc_component_idx = Some(component_idx);
            }
        }

        ectx.owned_instances = (0..ectx.instances.len()).collect();
    }
}