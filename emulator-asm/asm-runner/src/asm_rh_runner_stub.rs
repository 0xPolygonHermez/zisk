use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
};

use crate::{AsmRHData, AsmRHHeader, AsmSharedMemory};
use anyhow::Result;

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
        _: Arc<Mutex<Option<AsmSharedMemory<AsmRHHeader>>>>,
        _: u64,
        _: i32,
        _: i32,
        _: Option<u16>,
        _: bool,
    ) -> Result<AsmRunnerRH> {
        Err(anyhow::anyhow!(
            "AsmRunnerRH::run() is not supported on this platform. Only Linux x86_64 is supported."
        ))
    }
}
