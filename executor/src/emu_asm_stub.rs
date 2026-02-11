use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use crate::{DeviceMetricsList, NestedDeviceMetricsList, StaticSMBundle};
use asm_runner::AsmRunnerMO;

use fields::PrimeField64;
use proofman_common::ProofCtx;
use sm_rom::RomSM;
use zisk_common::{io::ZiskStdin, EmuTrace, ExecutorStatsHandle, StatsScope};
use zisk_core::ZiskRom;

pub struct EmulatorAsm {}

impl EmulatorAsm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        _zisk_rom: Arc<ZiskRom>,
        _world_rank: i32,
        _local_rank: i32,
        _base_port: Option<u16>,
        _unlock_mapped_memory: bool,
        _chunk_size: u64,
        _rom_sm: Option<Arc<RomSM>>,
    ) -> Self {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }

    #[allow(clippy::type_complexity)]
    pub fn execute<F: PrimeField64>(
        &self,
        _stdin: &Mutex<ZiskStdin>,
        _pctx: &ProofCtx<F>,
        _sm_bundle: &StaticSMBundle<F>,
        _stats: &ExecutorStatsHandle,
        _caller_stats_scope: &StatsScope,
    ) -> (
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        u64,
    ) {
        unimplemented!("AsmRunner is only supported on Linux x86_64 platforms.");
    }
}
