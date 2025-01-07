use crate::Plan;

pub struct InstanceCtx {
    /// Plan for the current instance
    pub plan: Plan,

    /// Global Id of the current instance
    pub global_idx: usize,
}

impl InstanceCtx {
    pub fn new(global_idx: usize, plan: Plan) -> Self {
        Self { plan, global_idx }
    }
}

unsafe impl Send for InstanceCtx {}
