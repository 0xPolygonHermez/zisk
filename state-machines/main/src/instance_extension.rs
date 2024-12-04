use proofman_common::AirInstance;
use zisk_core::ZiskOperationType;
use ziskemu::EmuTraceStart;

pub struct InstanceExtensionCtx<F> {
    pub op_type: ZiskOperationType,
    pub emu_trace_start: EmuTraceStart,
    pub segment_id: Option<usize>,
    pub instance_global_idx: usize,
    pub air_instance: Option<AirInstance<F>>,
}

impl<F: Default + Clone> InstanceExtensionCtx<F> {
    pub fn new(
        op_type: ZiskOperationType,
        emu_trace_start: EmuTraceStart,
        segment_id: Option<usize>,
        instance_global_idx: usize,
        air_instance: Option<AirInstance<F>>,
    ) -> Self {
        Self {
            op_type,
            emu_trace_start,
            instance_global_idx,
            segment_id,
            air_instance,
        }
    }
}
