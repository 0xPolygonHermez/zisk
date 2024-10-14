use zisk_core::ZiskOperationType;

pub struct EmuSegment {
    pub start: usize,
    pub end: usize,
    pub segment_type: ZiskOperationType,
    pub num_insts: usize,
}

impl EmuSegment {
    pub fn new(
        start: usize,
        end: usize,
        segment_type: ZiskOperationType,
        num_insts: usize,
    ) -> Self {
        Self { start, end, segment_type, num_insts }
    }
}
