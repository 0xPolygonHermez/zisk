use std::ffi::c_void;

use crate::AsmRHData;
use anyhow::Result;
use zisk_common::ExecutorStatsHandle;

pub struct PreloadedRH {}

// This struct is used to run the assembly code in a separate process and generate the ROM histogram.
pub struct AsmRunnerRH {
    pub asm_rowh_output: AsmRHData,
}

unsafe impl Send for AsmRunnerRH {}
unsafe impl Sync for AsmRunnerRH {}

impl AsmRunnerRH {
    pub fn new(
        _shmem_output_name: String,
        _mapped_ptr: *mut c_void,
        _asm_rowh_output: AsmRHData,
    ) -> Self {
        panic!(
            "AsmRunnerRH::new() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn run(
        _: &mut Option<PreloadedRH>,
        _: u64,
        _: i32,
        _: i32,
        _: Option<u16>,
        _: bool,
        _: ExecutorStatsHandle,
    ) -> Result<AsmRunnerRH> {
        Err(anyhow::anyhow!(
            "AsmRunnerRH::run() is not supported on this platform. Only Linux x86_64 is supported."
        ))
    }
}
