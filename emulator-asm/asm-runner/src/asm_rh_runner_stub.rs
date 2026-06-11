//! Non-Linux-x86_64 stub: placeholder types whose methods error/panic. Off the
//! supported platform these are never exercised, so they carry no docs.
#![allow(missing_docs)]

use crate::AsmRHData;
use anyhow::Result;
use zisk_common::ExecutorStatsHandle;

pub struct RHShmemReader {}

// This struct is used to run the assembly code in a separate process and generate the ROM histogram.
pub struct AsmRunnerRH {
    pub asm_rowh_output: AsmRHData,
}

impl AsmRunnerRH {
    /// Constructs an `AsmRunnerRH` from a histogram payload. Platform-agnostic — the
    /// Linux-only part is `run`, which spawns a child process and reads shared memory.
    /// This signature mirrors the Linux-x86_64 impl so callers (including tests) compile
    /// uniformly.
    pub fn new(asm_rowh_output: AsmRHData) -> Self {
        AsmRunnerRH { asm_rowh_output }
    }

    pub fn run(
        _: &mut Option<RHShmemReader>,
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
