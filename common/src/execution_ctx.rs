use crate::AirInstance;
#[allow(dead_code)]
/// Represents the context when executing a witness computer plugin
pub struct ExecutionCtx {
    /// If true, the plugin must generate the public outputs
    pub public_output: bool,

    pub discovering: bool,

    pub instances: Vec<AirInstance>,
    pub owned_instances: Vec<usize>,
}

impl ExecutionCtx {
    pub fn builder() -> ExecutionCtxBuilder {
        ExecutionCtxBuilder::new()
    }
}

pub struct ExecutionCtxBuilder {
    public_output: bool,
    pub discovering: bool,
}

impl ExecutionCtxBuilder {
    pub fn new() -> Self {
        ExecutionCtxBuilder { public_output: true, discovering: false }
    }

    pub fn is_discovery_execution(mut self) -> Self {
        self.discovering = true;
        self
    }

    pub fn build(self) -> ExecutionCtx {
        ExecutionCtx {
            public_output: self.public_output,
            discovering: self.discovering,
            instances: vec![],
            owned_instances: vec![],
        }
    }
}
