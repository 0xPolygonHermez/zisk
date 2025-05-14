use std::ffi::c_void;
use std::path::Path;

use crate::AsmRunnerOptions;
use zisk_common::{ChunkId, EmuTrace};

pub trait Task: Send + Sync + 'static {
    type Output: Send + 'static;
    fn execute(&self) -> Self::Output;
}

pub type TaskFactory<'a, T> = Box<dyn Fn(ChunkId, EmuTrace) -> T + Send + Sync + 'a>;

#[derive(Debug)]
pub enum MinimalTraces {
    None,
    EmuTrace(Vec<EmuTrace>),
    AsmEmuTrace(AsmRunnerMT),
}

#[derive(Debug)]
pub struct AsmRunnerMT {
    pub vec_chunks: Vec<EmuTrace>,
}

impl AsmRunnerMT {
    pub fn new(
        _shmem_output_name: String,
        _mapped_ptr: *mut c_void,
        _vec_chunks: Vec<EmuTrace>,
    ) -> Self {
        panic!(
            "AsmRunnerMT::new() is not supported on this platform. Only Linux x86_64 is supported."
        )
    }

    pub fn run(
        _ziskemuasm_path: &Path,
        _inputs_path: &Path,
        _shm_size: u64,
        _chunk_size: u64,
        _options: AsmRunnerOptions,
    ) -> AsmRunnerMT {
        panic!(
            "AsmRunnerMT::run() is not supported on this platform. Only Linux x86_64 is supported."
        )
    }

    pub fn run_and_count<T: Task>(
        _ziskemuasm_path: &Path,
        _inputs_path: &Path,
        _shm_size: u64,
        _chunk_size: u64,
        _options: AsmRunnerOptions,
        _task_factory: TaskFactory<T>,
    ) -> (AsmRunnerMT, Vec<T::Output>) {
        panic!("AsmRunnerMT::run_and_count() is not supported on this platform. Only Linux x86_64 is supported.")
    }
}

unsafe impl Send for AsmRunnerMT {}
unsafe impl Sync for AsmRunnerMT {}
