use crate::{MemInput, MemPreviousSegment};
use mem_common::MemModuleSegmentCheckPoint;
use proofman_common::{AirInstance, ProofmanResult};
#[cfg(any(feature = "debug_mem", feature = "debug_mem_offsets"))]
use std::{
    fs::File,
    io::{BufWriter, Write},
};
use zisk_common::SegmentId;

impl MemInput {
    pub fn new(addr: u32, is_write: bool, step: u64, value: u64) -> Self {
        MemInput { addr, is_write, step, value, is_internal: false }
    }
    pub fn new_internal(addr: u32, step: u64, value: u64) -> Self {
        MemInput { addr, is_write: false, step, value, is_internal: true }
    }
}

pub trait MemModule<F: Clone>: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn compute_witness(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        packed: bool,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>>;
    fn get_addr_range(&self) -> (u32, u32);
    fn is_dual(&self) -> bool;
    fn get_mem_name(&self) -> &str;
}

#[cfg(any(feature = "debug_mem", feature = "debug_mem_offsets"))]
pub fn save_offsets_to_file(seg: &MemModuleSegmentCheckPoint, file_name: &str) {
    println!("[MemDebug] saving offsets to {} .....", file_name);
    let file = File::create(file_name).unwrap();
    let mut writer = BufWriter::new(file);
    let base = seg.offsets_base_addr as u64;
    for index in 0..seg.addr_range_slots {
        let addr = index as u64 * 8 + base;
        let value = seg.offset_at(index);
        writeln!(writer, "{} {:#010X} {}", index, addr, value).unwrap();
    }
    println!("[MemDebug] done");
}
