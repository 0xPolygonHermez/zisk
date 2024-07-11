use crate::AirInstanceCtx;

#[derive(Debug)]
pub struct AirInstance {
    air_group_id: usize,
    air_id: usize,
    air_instance_id: usize,
    pub meta: Option<Box<dyn std::any::Any>>,
}

impl AirInstance {
    pub fn new(air_group_id: usize, air_id: usize, instance_id: usize) -> Self {
        AirInstance { air_group_id, air_id, air_instance_id: instance_id, meta: None }
    }
}

impl Into<AirInstanceCtx> for &AirInstance {
    fn into(self) -> AirInstanceCtx {
        AirInstanceCtx::new(self.air_group_id, self.air_id, self.air_instance_id)
    }
}

pub enum AirInstancesSet {
    None,
    All,
    Set(Vec<usize>),
}
