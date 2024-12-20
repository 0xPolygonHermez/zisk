use std::any::Any;

use crate::{BusDeviceMetrics, InstanceType};

pub type ChunkId = usize;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CollectInfoSkip {
    pub skip: u64,
}

impl CollectInfoSkip {
    pub fn new(skip: u64) -> Self {
        CollectInfoSkip { skip }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckPoint {
    None,
    Single(ChunkId),
    Multiple(Vec<ChunkId>),
}

#[derive(Debug)]
pub struct Plan {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub segment_id: Option<usize>,
    pub instance_type: InstanceType,
    pub check_point: CheckPoint,
    pub collect_info: Option<Box<dyn Any>>,
    pub meta: Option<Box<dyn Any>>,
}

impl Plan {
    pub fn new(
        airgroup_id: usize,
        air_id: usize,
        segment_id: Option<usize>,
        instance_type: InstanceType,
        check_point: CheckPoint,
        collect_info: Option<Box<dyn Any>>,
        meta: Option<Box<dyn Any>>,
    ) -> Self {
        Plan { airgroup_id, air_id, segment_id, instance_type, check_point, collect_info, meta }
    }
}

pub trait Planner {
    fn plan(&self, counter: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan>;
}
