use std::any::Any;

use crate::Surveyor;

pub type ChunkId = usize;

#[derive(Debug, PartialEq)]
pub struct CheckPoint {
    pub chunk_id: ChunkId,
    // offset inside the chunk to start the trace. The offset corresponds to the number of instructions that the sorveyor has seen.
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
    pub checkpoint: CheckPoint,
    pub meta: Option<Box<dyn Any>>,
}

impl Plan {
    pub fn new(
        airgroup_id: usize,
        air_id: usize,
        segment_id: Option<usize>,
        checkpoint: CheckPoint,
        meta: Option<Box<dyn Any>>,
    ) -> Self {
        Plan { airgroup_id, air_id, segment_id, checkpoint, meta }
    }
}

pub trait Planner {
    fn plan(&self, surveys: Vec<(ChunkId, Box<dyn Surveyor>)>) -> Vec<Plan>;
}
