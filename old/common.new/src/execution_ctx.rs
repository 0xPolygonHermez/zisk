use crate::AirInstancesSet;

#[allow(dead_code)]
/// Represents the context when executing a witness computer plugin
pub struct ExecutionCtx {
    /// If true, the plugin must generate the public outputs
    pub public_output: bool,

    /// If true, the plugin must generate the air instances map
    pub air_instances_map: bool,

    /// If Some, it must generate the witness computation for the given set of air instances
    pub witness_computation: Option<AirInstancesSet>,
}

impl ExecutionCtx {
    pub fn builder() -> ExecutionCtxBuilder {
        ExecutionCtxBuilder::new()
    }
}

pub struct ExecutionCtxBuilder {
    public_output: bool,
    air_instances_map: bool,
    witness_computation: Option<AirInstancesSet>,
}

impl ExecutionCtxBuilder {
    pub fn new() -> Self {
        ExecutionCtxBuilder { public_output: false, air_instances_map: false, witness_computation: None }
    }

    pub fn with_public_output(mut self) -> Self {
        self.public_output = true;
        self
    }

    pub fn with_air_instances_map(mut self) -> Self {
        self.air_instances_map = true;
        self
    }

    pub fn with_instances(mut self, instances_set: AirInstancesSet) -> Self {
        self.witness_computation = Some(instances_set);
        self
    }

    pub fn with_all_instances(mut self) -> Self {
        self.witness_computation = Some(AirInstancesSet::All);
        self
    }

    pub fn build(self) -> ExecutionCtx {
        ExecutionCtx {
            public_output: self.public_output,
            air_instances_map: self.air_instances_map,
            witness_computation: self.witness_computation,
        }
    }
}
