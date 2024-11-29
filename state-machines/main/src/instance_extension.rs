use std::sync::Arc;

use proofman_common::AirInstance;
use sm_common::CheckPoint;
// use sm_common::StateMachine;
use ziskemu::EmuTraceStart;

#[derive(Default)]
pub struct InstanceExtensionCtx<F> {
    pub prover_buffer: Vec<F>,
    pub offset: u64,
    pub segment_id: Option<usize>,
    pub instance_global_idx: usize,
    pub air_instance: Option<AirInstance<F>>,
}

impl<F: Default + Clone> InstanceExtensionCtx<F> {
    pub fn new(
        prover_buffer: Vec<F>,
        offset: u64,
        segment_id: Option<usize>,
        instance_global_idx: usize,
        air_instance: Option<AirInstance<F>>,
    ) -> Self {
        Self {
            prover_buffer,
            offset,
            instance_global_idx,
            segment_id,
            air_instance,
        }
    }
}

// pub struct InstanceExtensionCtx2<F> {
//     pub sm: Arc<dyn StateMachine<F>>,
//     pub prover_buffer: Vec<F>,
//     pub offset: u64,
//     pub emu_trace_start_step: Option<(EmuTraceStart, u64)>,
//     pub segment_id: Option<usize>,
//     pub instance_global_idx: usize,
//     pub air_instance: Option<AirInstance<F>>,
// }

// impl<F: Default> InstanceExtensionCtx2<F> {
//     pub fn new(
//         sm: Arc<dyn StateMachine<F>>,
//         prover_buffer: Vec<F>,
//         offset: u64,
//         emu_trace_start_step: Option<(EmuTraceStart, u64)>,
//         segment_id: Option<usize>,
//         instance_global_idx: usize,
//         air_instance: Option<AirInstance<F>>,
//     ) -> Self {
//         Self {
//             sm,
//             prover_buffer,
//             offset,
//             emu_trace_start_step,
//             instance_global_idx,
//             segment_id,
//             air_instance,
//         }
//     }
// }
