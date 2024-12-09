use crate::Plan;

pub struct InstanceExpanderCtx {
    pub plan: Plan,
    pub instance_global_idx: usize,
}

impl InstanceExpanderCtx {
    pub fn new(instance_global_idx: usize, plan: Plan) -> Self {
        Self { plan, instance_global_idx }
    }
}

unsafe impl Send for InstanceExpanderCtx {}
