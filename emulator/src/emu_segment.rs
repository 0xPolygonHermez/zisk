use zisk_core::ZiskOperationType;

use crate::EmuTraceCheckPoint;

#[derive(Debug, Clone)]
pub struct EmuStartingPoint {
    pub op_type: ZiskOperationType,
    pub emu_trace_start: EmuTraceCheckPoint,
}

impl EmuStartingPoint {
    pub fn new(segment_type: ZiskOperationType, pc: u64, sp: u64, c: u64, step: u64) -> Self {
        Self {
            op_type: segment_type,
            emu_trace_start: EmuTraceCheckPoint {
                pc,
                sp,
                c,
                step,
                regs: [0; 32],
                mem_reads_index: 0,
            },
        }
    }
}
