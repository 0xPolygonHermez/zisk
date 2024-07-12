use crate::AirInstance;

#[allow(dead_code)]
/// Represents the context when executing a witness computer plugin
pub struct ExecutionCtx {
    /// If true, the plugin must generate the public outputs
    pub public_output: bool,

    pub is_discovery_execution: bool,

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
    pub is_discovery_execution: bool,
}

impl ExecutionCtxBuilder {
    pub fn new() -> Self {
        ExecutionCtxBuilder { public_output: true, is_discovery_execution: false }
    }

    pub fn is_discovery_execution(mut self) -> Self {
        self.is_discovery_execution = true;
        self
    }

    pub fn build(self) -> ExecutionCtx {
        ExecutionCtx {
            public_output: self.public_output,
            is_discovery_execution: self.is_discovery_execution,
            instances: vec![],
            owned_instances: vec![],
        }
    }
}
