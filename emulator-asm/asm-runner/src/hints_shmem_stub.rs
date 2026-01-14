use anyhow::Result;
use zisk_common::io::StreamSink;

/// HintsShmem struct manages the writing of processed precompile hints to shared memory.
pub struct HintsShmem;

impl HintsShmem {
    pub fn new(
        _base_port: Option<u16>,
        _local_rank: i32,
        _unlock_mapped_memory: bool,
    ) -> Result<Self> {
        unreachable!(
            "HintsShmem::new() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}

impl StreamSink for HintsShmem {
    fn submit(&self, _processed: Vec<u64>) -> anyhow::Result<()> {
        unreachable!(
            "HintsShmem::submit() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}
