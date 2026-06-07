use zisk_common::{EmuTrace, ExecutorStatsHandle};

use std::ffi::c_void;
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Result;

pub struct PreloadedMT {}

// This struct is used to run the assembly code in a separate process and generate minimal traces.
#[derive(Debug)]
pub struct AsmRunnerMT {
    pub vec_chunks: Vec<EmuTrace>,
}

impl AsmRunnerMT {
    pub fn new(_: String, _: *mut c_void, _: Vec<EmuTrace>) -> Self {
        panic!(
            "AsmRunnerMT::new() is not supported on this platform. Only Linux x86_64 is supported."
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_and_count<F, R>(
        _: &mut PreloadedMT,
        _: u64,
        _: u64,
        _: F,
        _: R,
        _: i32,
        _: i32,
        _: Option<u16>,
        _: ExecutorStatsHandle,
    ) -> Result<Vec<Arc<EmuTrace>>>
    where
        F: FnMut(usize, Arc<EmuTrace>),
        R: FnOnce() -> Result<()>,
    {
        Err(anyhow::anyhow!("AsmRunnerMT::run_and_count() is not supported on this platform. Only Linux x86_64 is supported."))
    }
}
