use std::{sync::Arc, thread::JoinHandle};

use crate::{DeviceMetricsList, NestedDeviceMetricsList, StaticSMBundle};
use anyhow::Result;
use asm_runner::{AsmRunnerMO, AsmRunnerRH, HintsShmem};
use precompiles_hints::HintsProcessor;

use crate::AsmResources;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use zisk_common::{
    io::{StreamSource, ZiskStdin},
    AsmExecutionInfo, EmuTrace, ExecutorStatsHandle, StatsScope,
};
use zisk_core::ZiskRom;

pub struct EmulatorAsm {}

impl EmulatorAsm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(_chunk_size: u64) -> Self {
        Self {}
    }

    #[allow(clippy::type_complexity)]
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
    ) -> Result<(
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<Result<AsmRunnerMO>>>,
        Option<JoinHandle<Result<AsmRunnerRH>>>,
        u64,
    )> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn set_asm_resources(&self, _asm_resources: Arc<AsmResources>) -> Result<()> {
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

    pub fn get_hints_processor(&self) -> Result<Arc<HintsProcessor<HintsShmem>>> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn set_active_services(&self, _is_first_partition: bool) -> Result<()> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn set_hints_stream_src(&self, _stream: StreamSource) -> Result<()> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn set_inputs_stream_src(&self, _stream: StreamSource) -> Result<()> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn submit_hint_direct(&self, _data: &[u64]) -> Result<()> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn append_raw_input(&self, _bytes: &[u8]) -> Result<()> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }
}
