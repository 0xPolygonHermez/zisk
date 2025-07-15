use serde::{Deserialize, Serialize};
use zisk_common::SegmentId;

use crate::mem_sm::MemPreviousSegment;

#[allow(dead_code)]
fn format_u64_hex(value: u64) -> String {
    let hex_str = format!("{value:016x}");
    hex_str
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("_")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemAlignInput {
    pub addr: u32,
    pub is_write: bool,
    pub width: u8,
    pub step: u64,
    pub value: u64,
    pub mem_values: [u64; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemInput {
    pub addr: u32,      // address in word native format means byte_address / MEM_BYTES
    pub is_write: bool, // it's a write operation
    pub step: u64,      // mem_step = f(main_step, main_step_offset)
    pub value: u64,     // value to read or write
}

pub struct MemInstanceInfo {
    pub segment_id: SegmentId,
    pub is_last_segment: bool,
    pub previous_segment: MemPreviousSegment,
}
