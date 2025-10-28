use zisk_common::{ChunkId, EmuTrace, ExecutorStatsHandle};

use std::ffi::c_void;
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Result;
pub trait Task: Send + Sync + 'static {
    type Output: Send + 'static;
    fn execute(self) -> Self::Output;
}

pub type TaskFactory<'a, T> = Box<dyn Fn(ChunkId, Arc<EmuTrace>) -> T + Send + Sync + 'a>;

#[derive(Debug)]
pub enum MinimalTraces {
    None,
    EmuTrace(Vec<EmuTrace>),
    AsmEmuTrace(AsmRunnerMT),
}

pub struct PreloadedMT {}

// This struct is used to run the assembly code in a separate process and generate minimal traces.
#[derive(Debug)]
pub struct AsmRunnerMT {
    pub vec_chunks: Vec<EmuTrace>,
}

unsafe impl Send for AsmRunnerMT {}
unsafe impl Sync for AsmRunnerMT {}

impl AsmRunnerMT {
    pub fn new(_: String, _: *mut c_void, _: Vec<EmuTrace>) -> Self {
        panic!(
            "AsmRunnerMT::new() is not supported on this platform. Only Linux x86_64 is supported."
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_and_count<T: Task>(
        _: &mut PreloadedMT,
        _: u64,
        _: u64,
        _: TaskFactory<T>,
        _: i32,
        _: i32,
        _: Option<u16>,
        _: ExecutorStatsHandle,
    ) -> Result<(AsmRunnerMT, Vec<T::Output>)> {
        Err(anyhow::anyhow!("AsmRunnerMT::run_and_count() is not supported on this platform. Only Linux x86_64 is supported."))
    }
}
