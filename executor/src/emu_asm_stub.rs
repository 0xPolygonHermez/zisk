use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use crate::{DeviceMetricsList, NestedDeviceMetricsList, StaticSMBundle};
use asm_runner::{AsmRunnerMO, AsmRunnerRH};

use crate::AsmResources;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use sm_rom::RomSM;
use zisk_common::{io::ZiskStdin, AsmExecutionInfo, EmuTrace, ExecutorStatsHandle, StatsScope};
use zisk_core::ZiskRom;

pub struct EmulatorAsm {}

impl EmulatorAsm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        _world_rank: i32,
        _local_rank: i32,
        _unlock_mapped_memory: bool,
        _chunk_size: u64,
        _rom_sm: Option<Arc<RomSM>>,
        _verbose_mode: proofman_common::VerboseMode,
    ) -> Self {
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
    ) -> (
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        Option<JoinHandle<AsmRunnerRH>>,
        u64,
    ) {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn set_asm_resources(&self, _asm_resources: AsmResources) {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn set_rh_data(&self, _rh_data: AsmRunnerRH) {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn get_chunk_size(&self) -> u64 {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn reset_hints_stream(&self) {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    pub fn get_asm_execution_info(&self) -> Option<AsmExecutionInfo> {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }
}
