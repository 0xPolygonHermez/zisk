use zisk_core::{ZiskOperationType, ZiskOperationTypeVariants};

use crate::EmuTraceStart;

#[derive(Debug, Default)]
pub struct EmuSegments {
    pub segments: Vec<EmuSegment>,
    pub num_segments: [u64; ZiskOperationTypeVariants],
    pub total_steps: [u64; ZiskOperationTypeVariants],
}

impl EmuSegments {
    pub fn add(&mut self, segment_type: ZiskOperationType, pc: u64, sp: u64, c: u64, step: u64) {
        self.segments.push(EmuSegment::new(segment_type.clone(), pc, sp, c, step));
        self.num_segments[segment_type.clone() as usize] += 1;
    }
}

#[derive(Debug, Clone)]
pub struct EmuSegment {
    pub segment_type: ZiskOperationType,
    pub emu_trace_start: EmuTraceStart,
}

impl EmuSegment {
    pub fn new(segment_type: ZiskOperationType, pc: u64, sp: u64, c: u64, step: u64) -> Self {
        Self { segment_type, emu_trace_start: EmuTraceStart { pc, sp, c, step } }
    }
}
