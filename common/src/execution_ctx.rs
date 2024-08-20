use std::sync::Arc;

use crate::BufferAllocator;
#[allow(dead_code)]
/// Represents the context when executing a witness computer plugin
pub struct ExecutionCtx {
    /// If true, the plugin must generate the public outputs
    pub public_output: bool,
    pub buffer_allocator: Arc<dyn BufferAllocator>,
}

impl ExecutionCtx {
    pub fn builder() -> ExecutionCtxBuilder {
        ExecutionCtxBuilder::new()
    }
}

pub struct ExecutionCtxBuilder {
    public_output: bool,
    buffer_allocator: Option<Arc<dyn BufferAllocator>>,
}

impl Default for ExecutionCtxBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionCtxBuilder {
    pub fn new() -> Self {
        ExecutionCtxBuilder { public_output: true, buffer_allocator: None }
    }

    pub fn with_buffer_allocator(mut self, buffer_allocator: Arc<dyn BufferAllocator>) -> Self {
        self.buffer_allocator = Some(buffer_allocator);
        self
    }

    pub fn build(self) -> ExecutionCtx {
        if self.buffer_allocator.is_none() {
            panic!("Buffer allocator is required");
        }

        ExecutionCtx { public_output: self.public_output, buffer_allocator: self.buffer_allocator.unwrap() }
    }
}
