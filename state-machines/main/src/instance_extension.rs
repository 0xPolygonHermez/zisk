use proofman_common::AirInstance;

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
        Self { prover_buffer, offset, instance_global_idx, segment_id, air_instance }
    }
}
