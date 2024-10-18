use zisk_core::{ZiskOperationType, ZISK_OPERATION_TYPE_VARIANTS};

use crate::EmuTraceStart;

#[derive(Debug, Default)]
pub struct EmuStartingPoints {
    pub points: Vec<EmuStartingPoint>,
    pub num_points: [u64; ZISK_OPERATION_TYPE_VARIANTS],
    pub total_steps: [u64; ZISK_OPERATION_TYPE_VARIANTS],
}

impl EmuStartingPoints {
    pub fn add(&mut self, op_type: ZiskOperationType, pc: u64, sp: u64, c: u64, step: u64) {
        self.points.push(EmuStartingPoint::new(op_type, pc, sp, c, step));
        self.num_points[op_type as usize] += 1;
    }
}

#[derive(Debug, Clone)]
pub struct EmuStartingPoint {
    pub op_type: ZiskOperationType,
    pub emu_trace_start: EmuTraceStart,
}

impl EmuStartingPoint {
    pub fn new(segment_type: ZiskOperationType, pc: u64, sp: u64, c: u64, step: u64) -> Self {
        Self { op_type: segment_type, emu_trace_start: EmuTraceStart { pc, sp, c, step } }
    }
}
