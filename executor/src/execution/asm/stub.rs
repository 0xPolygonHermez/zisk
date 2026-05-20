//! Non-x86_64 stub of [`EmulatorAsm`].
//!
//! Mirrors the inherent surface of the real [`super::emulator::EmulatorAsm`]
//! so [`crate::execution::ExecutionPhase`] can hold an
//! `EmulatorBackend::Asm(EmulatorAsm)` variant on every target without
//! `#[cfg]` gates in callers. Any method call panics with a clear
//! "Linux x86_64 only" message — the ASM emulator is genuinely
//! unavailable here, so failing fast is honest.

#![allow(missing_docs)]

use std::sync::Arc;

use anyhow::Result;
use asm_runner::HintsShmem;
use fields::PrimeField64;
use precompiles_hints::HintsProcessor;
use proofman_common::ProofCtx;
use zisk_common::{
    io::{StreamSource, ZiskStdin},
    AsmExecutionInfo, ExecutorStatsHandle, StatsScope,
};
use zisk_core::ZiskRom;

use super::AsmResources;
use crate::execution::output::ExecutionOutput;
use crate::sm::StaticSMBundle;

fn unsupported() -> ! {
    panic!("ASM emulator backend is only available on Linux x86_64");
}

/// Stub `EmulatorAsm` for targets without ASM support.
pub struct EmulatorAsm;

impl EmulatorAsm {
    pub fn new(_chunk_size: u64) -> Self {
        unsupported()
    }

    pub fn get_asm_execution_info(&self) -> Result<Option<AsmExecutionInfo>> {
        unsupported()
    }

    pub fn set_asm_resources(&self, _: Arc<AsmResources>) -> Result<()> {
        unsupported()
    }

    pub fn submit_hint_direct(&self, _: &[u64]) -> Result<()> {
        unsupported()
    }

    pub fn append_raw_input(&self, _: &[u8]) -> Result<()> {
        unsupported()
    }

    pub fn set_hints_stream_src(&self, _: StreamSource) -> Result<()> {
        unsupported()
    }

    pub fn set_inputs_stream_src(&self, _: StreamSource) -> Result<()> {
        unsupported()
    }

    pub fn get_hints_processor(&self) -> Result<Arc<HintsProcessor<HintsShmem>>> {
        unsupported()
    }

    pub fn set_active_services(&self, _: bool) -> Result<()> {
        unsupported()
    }

    pub fn reset(&self) -> Result<()> {
        unsupported()
    }

    pub fn signal_cancellation(&self) -> Result<()> {
        unsupported()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute<F: PrimeField64>(
        &self,
        _zisk_rom: &ZiskRom,
        _stdin: &ZiskStdin,
        _pctx: &ProofCtx<F>,
        _sm_bundle: &StaticSMBundle<F>,
        _use_hints: bool,
        _stats: &ExecutorStatsHandle,
        _caller_stats_scope: &StatsScope,
    ) -> Result<ExecutionOutput> {
        unsupported()
    }
}
