use std::sync::Arc;

use crate::{BufferAllocator, VerboseMode};
#[allow(dead_code)]
/// Represents the context when executing a witness computer plugin
pub struct ExecutionCtx {
    /// If true, the plugin must generate the public outputs
    pub public_output: bool,
    pub buffer_allocator: Arc<dyn BufferAllocator>,
    pub verbose_mode: VerboseMode,
}

impl ExecutionCtx {
    pub fn builder() -> ExecutionCtxBuilder {
        ExecutionCtxBuilder::new()
    }
}

pub struct ExecutionCtxBuilder {
    public_output: bool,
    buffer_allocator: Option<Arc<dyn BufferAllocator>>,
    verbose_mode: VerboseMode,
}

impl Default for ExecutionCtxBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionCtxBuilder {
    pub fn new() -> Self {
        ExecutionCtxBuilder { public_output: true, buffer_allocator: None, verbose_mode: VerboseMode::Info }
    }

    pub fn with_buffer_allocator(mut self, buffer_allocator: Arc<dyn BufferAllocator>) -> Self {
        self.buffer_allocator = Some(buffer_allocator);
        self
    }

    pub fn with_verbose_mode(mut self, verbose_mode: VerboseMode) -> Self {
        self.verbose_mode = verbose_mode;
        self
    }

    pub fn build(self) -> ExecutionCtx {
        if self.buffer_allocator.is_none() {
            panic!("Buffer allocator is required");
        }

        ExecutionCtx {
            public_output: self.public_output,
            buffer_allocator: self.buffer_allocator.unwrap(),
            verbose_mode: self.verbose_mode,
        }
    }
}
