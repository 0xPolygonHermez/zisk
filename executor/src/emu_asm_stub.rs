use std::sync::Mutex;

use crate::{DeviceMetricsList, EmulatorResult, NestedDeviceMetricsList, StaticSMBundle};
use anyhow::Result;

use fields::PrimeField64;
use proofman_common::ProofCtx;
use zisk_common::{
    io::{StreamSource, ZiskStdin},
    AsmExecutionInfo, ExecutorStatsHandle, StatsScope,
};
use zisk_core::ZiskRom;

/// Stub for `AsmResources` on non-x86_64 platforms.
#[derive(Clone, Debug)]
pub struct AsmResources;

pub struct EmulatorAsm {}

impl EmulatorAsm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(_chunk_size: u64) -> Self {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    pub fn execute<F: PrimeField64>(
        &self,
        _zisk_rom: &ZiskRom,
        _stdin: &Mutex<ZiskStdin>,
        _pctx: &ProofCtx<F>,
        _sm_bundle: &StaticSMBundle<F>,
        _use_hints: bool,
        _stats: &ExecutorStatsHandle,
        _caller_stats_scope: &StatsScope,
    ) -> Result<EmulatorResult> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn set_asm_resources(&self, _asm_resources: AsmResources) -> Result<()> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn get_chunk_size(&self) -> u64 {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn reset(&self) -> Result<()> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn get_asm_execution_info(&self) -> Result<Option<AsmExecutionInfo>> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn use_hints(&self) -> Result<bool> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn get_hints_processor(
        &self,
    ) -> Result<Option<std::sync::Arc<dyn zisk_common::io::StreamProcessor>>> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn set_active_services(&self, _is_first_partition: bool) -> Result<()> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn set_hints_stream_src(&self, _stream: StreamSource) -> Result<()> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn submit_hint_direct(&self, _data: &[u64]) -> Result<()> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn append_raw_input(&self, _bytes: &[u8]) -> Result<()> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }
}
