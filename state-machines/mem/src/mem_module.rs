use crate::{MemInput, MemPreviousSegment};
use proofman_common::{AirInstance, ProofmanResult};
#[cfg(feature = "debug_mem")]
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
        offset_base_addr: u32,
        offsets: &[u32],
    ) -> ProofmanResult<AirInstance<F>>;
    fn get_addr_range(&self) -> (u32, u32);
    fn is_dual(&self) -> bool;
    fn get_mem_name(&self) -> &str;
}

pub fn get_previous_addr_w(
    offsets: &[u32],
    addr_index: usize,
    offset_base_addr_w: u32,
) -> Option<u64> {
    let ref_offset = offsets[addr_index];
    (0..addr_index)
        .rev()
        .find(|&i| offsets[i] != ref_offset)
        .map(|prev_addr_offset| offset_base_addr_w as u64 + prev_addr_offset as u64)
}

#[cfg(feature = "debug_mem")]
pub fn save_offsets_to_file(offset_base_addr: u32, offsets: &[u32], file_name: &str) {
    println!("[MemDebug] saving offsets to {} .....", file_name);
    let file = File::create(file_name).unwrap();
    let mut writer = BufWriter::new(file);
    for (index, &value) in offsets.iter().enumerate() {
        let addr = index as u64 * 8 + offset_base_addr as u64;
        writeln!(writer, "{} {:#010X} {}", index, addr, value).unwrap();
    }
    println!("[MemDebug] done");
}
