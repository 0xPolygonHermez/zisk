use std::ffi::c_void;
use std::path::Path;

use crate::{AsmRHData, AsmRunnerOptions};

#[derive(Debug)]
pub struct AsmRunnerRomH {
    pub asm_rowh_output: AsmRHData,
}

unsafe impl Send for AsmRunnerRomH {}
unsafe impl Sync for AsmRunnerRomH {}

impl AsmRunnerRomH {
    pub fn new(
        _shmem_output_name: String,
        _mapped_ptr: *mut c_void,
        _asm_rowh_output: AsmRHData,
    ) -> Self {
        panic!("AsmRunnerRomH::new() is not supported on this platform. Only Linux x86_64 is supported.");
    }

    pub fn run(
        _rom_asm_path: &Path,
        _inputs_path: Option<&Path>,
        _shm_size: u64,
        _options: AsmRunnerOptions,
    ) -> AsmRunnerRomH {
        panic!("AsmRunnerRomH::run() is not supported on this platform. Only Linux x86_64 is supported.");
    }
}
