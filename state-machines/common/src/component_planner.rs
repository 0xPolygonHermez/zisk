use std::any::Any;

use crate::{BusDeviceMetrics, InstanceType};

pub type ChunkId = usize;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CheckPoint {
    pub chunk_id: ChunkId,
    // offset inside the chunk to start the trace. The offset corresponds to the number of
    // instructions that the sorveyor has seen.
    pub skip: u64,
}

impl CheckPoint {
    pub fn new(chunk_id: ChunkId, offset: u64) -> Self {
        CheckPoint { chunk_id, skip: offset }
    }
}

#[derive(Debug)]
pub struct Plan {
    pub airgroup_id: usize,
    pub air_id: usize,
    pub segment_id: Option<usize>,
    pub instance_type: InstanceType,
    pub check_point: Option<CheckPoint>,
    pub meta: Option<Box<dyn Any>>,
}

impl Plan {
    pub fn new(
        airgroup_id: usize,
        air_id: usize,
        segment_id: Option<usize>,
        instance_type: InstanceType,
        check_point: Option<CheckPoint>,
        meta: Option<Box<dyn Any>>,
    ) -> Self {
        Plan { airgroup_id, air_id, segment_id, instance_type, check_point, meta }
    }
}

pub trait Planner {
    fn plan(&self, counter: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan>;
}
