use crate::Plan;

pub struct InstanceExpanderCtx {
    pub plan: Plan,
    pub global_idx: usize,
}

impl InstanceExpanderCtx {
    pub fn new(global_idx: usize, plan: Plan) -> Self {
        Self { plan, global_idx }
    }
}

unsafe impl Send for InstanceExpanderCtx {}
