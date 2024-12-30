use crate::Plan;

pub struct InstanceCtx {
    pub plan: Plan,
    pub global_idx: usize,
}

impl InstanceCtx {
    pub fn new(global_idx: usize, plan: Plan) -> Self {
        Self { plan, global_idx }
    }
}

unsafe impl Send for InstanceCtx {}
