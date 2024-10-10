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
    pub execution_type: ParEmuExecutionType,
}

impl ParEmuOptions {
    pub fn new(num_threads: usize, thread_id: usize, num_steps: usize, execution_type: ParEmuExecutionType) -> Self {
        Self {
            num_threads,
            thread_id,
            num_steps,
            execution_type,
        }
    }
}