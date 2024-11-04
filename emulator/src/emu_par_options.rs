use zisk_core::ZISK_OPERATION_TYPE_VARIANTS;

#[derive(Clone, Debug, PartialEq)]
pub enum ParEmuExecutionType {
    MainTrace,
    RequiredInputs,
}
#[derive(Clone, Debug)]
pub struct ParEmuOptions {
    pub num_threads: usize,
    pub thread_id: usize,
    pub num_steps: usize,
    pub segment_sizes: [u64; ZISK_OPERATION_TYPE_VARIANTS],
}

impl ParEmuOptions {
    pub fn new(
        num_threads: usize,
        thread_id: usize,
        num_steps: usize,
        segment_sizes: [u64; ZISK_OPERATION_TYPE_VARIANTS],
    ) -> Self {
        Self { num_threads, thread_id, num_steps, segment_sizes }
    }
}
