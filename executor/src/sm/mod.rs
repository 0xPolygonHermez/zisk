//! Bundle types — `StateMachines<F>` wrapper enum + `StaticSMBundle<F>`
//! registry.
//!
//! The bundle holds every state machine the executor exposes, both
//! built-in and precompile, indexed by Vec position. Per-side details:
//!
//! * **Built-ins** — `BuiltinSMs<F>` + `BuiltinCounters` +
//!   `BuiltinCollectors` live in [`builtins`].
//! * **Precompiles** — declarative registry in [`precompiles`]; emits
//!   `Precompiles<F>` + `PrecompileCounters<F>` +
//!   `PrecompileCollectors<F>` via the `register_precompiles!` macro
//!   defined in [`registry`].
//! * **Bus construction** — `StaticDataBus::from_bundle` /
//!   `StaticDataBusCollect::for_chunk` consume a bundle to build the
//!   per-phase data buses.
//! * **Canonical entry point** — `ZiskExecutor::new` in `executor.rs`.

mod builtins;
mod precompiles;
// `register_precompiles!` macro module; exported via `#[macro_export]`.
mod registry;

pub use builtins::*;
pub use precompiles::*;

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::{ExecutorError, ExecutorResult};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::ProofCtx;
use zisk_common::{Instance, InstanceCtx, Plan, Planner};
use zisk_pil::ZISK_AIRGROUP_ID;

use asm_runner::AsmRunnerRH;

use zisk_core::ZiskRom;

pub type SMType<F> = (SMAirType, StateMachines<F>);

pub enum StateMachines<F: PrimeField64> {
    Builtin(BuiltinSMs<F>),
    Precompile(Precompiles<F>),
}

impl<F: PrimeField64> StateMachines<F> {
    fn build_planner(&self, is_asm_emulator: bool) -> Box<dyn Planner> {
        match self {
            Self::Builtin(b) => b.build_planner(is_asm_emulator),
            Self::Precompile(p) => p.build_planner(),
        }
    }

    fn configure_instances(&self, pctx: &ProofCtx<F>, plans: &[Plan]) {
        match self {
            Self::Builtin(b) => b.configure_instances(pctx, plans),
            Self::Precompile(p) => p.configure_instances(pctx, plans),
        }
    }

    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match self {
            Self::Builtin(b) => b.build_instance(ictx),
            Self::Precompile(p) => p.build_instance(ictx),
        }
    }
}

pub struct StaticSMBundle<F: PrimeField64> {
    // Vec position is the SM's identity inside the bundle. External APIs
    // that need a usize key (`CountersChunkMetrics`, planning map) use
    // this position. Iteration order is insertion order.
    sm: Vec<SMType<F>>,

    /// Cached Vec position of the `RomSM` entry. ROM has no bus-side
    /// counter (`Planner::plan` is dead-coded for it); `plan_sec` looks
    /// up this position and feeds chunk_ids straight to
    /// [`sm_rom::RomPlanner::plan_for_chunks`].
    rom_position: usize,

    /// Cached Vec position of the `MemSM` entry. Looked up at construction
    /// so `extend_mem_plans` doesn't have to scan or hardcode a number.
    mem_position: usize,

    /// `true` when the bundle was built for the ASM emulator path
    /// (memory ops accounted out-of-band; ROM histogram via the RH service).
    /// Set once in [`Self::new`] from the `is_asm_emulator` argument;
    /// surfaces the value already threaded through `BuiltinSMs::all` so
    /// callers can read it via [`Self::is_asm`] instead of plumbing the
    /// same bool through every per-call API.
    is_asm: bool,

    std: Arc<Std<F>>,
}

impl<F: PrimeField64> StaticSMBundle<F> {
    /// Construct the bundle with the built-in SMs (Rom, Mem, Binary,
    /// Arith, Dma) created internally + the caller-supplied precompiles.
    pub fn new(
        std: Arc<Std<F>>,
        is_asm_emulator: bool,
        precompiles: Vec<(usize, Precompiles<F>)>,
    ) -> Self {
        let sm: Vec<SMType<F>> = BuiltinSMs::all(std.clone())
            .into_iter()
            .map(|(ids, b)| (ids, StateMachines::Builtin(b)))
            .chain(precompiles.into_iter().map(|(air_id, p)| {
                (vec![(ZISK_AIRGROUP_ID, air_id)], StateMachines::Precompile(p))
            }))
            .collect();

        let rom_position = sm
            .iter()
            .position(|(_, s)| matches!(s, StateMachines::Builtin(BuiltinSMs::RomSM(_))))
            .expect("RomSM must be in the bundle (constructed above)");

        let mem_position = sm
            .iter()
            .position(|(_, s)| matches!(s, StateMachines::Builtin(BuiltinSMs::MemSM(_))))
            .expect("MemSM must be in the bundle (constructed above)");

        Self { sm, rom_position, mem_position, is_asm: is_asm_emulator, std }
    }

    /// Returns `true` if the bundle was constructed for the ASM emulator
    /// path. Mirrors the `is_asm_emulator` argument passed to [`Self::new`].
    ///
    /// Used to remove the redundant `is_asm_emulator` parameter from
    /// non-hot-path APIs (`plan_sec`, `build_planner`, `from_bundle`) in
    /// later steps; the hot-path `StaticDataBus::from_bundle` keeps its
    /// explicit `bool` argument to avoid any codegen change.
    #[inline]
    pub fn is_asm(&self) -> bool {
        self.is_asm
    }

    /// Read-only view of all registered SMs in insertion order. Used
    /// by the bus-side wrapper structs (`BuiltinCounters::from_bundle`,
    /// etc.) to iterate without naming any specific precompile type.
    pub fn entries(&self) -> &[SMType<F>] {
        &self.sm
    }

    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> ExecutorResult<()> {
        for (_, sm) in self.sm.iter() {
            if let StateMachines::Builtin(BuiltinSMs::RomSM(rom_sm)) = sm {
                rom_sm.set_rom(zisk_rom.clone())?;
            }
        }
        Ok(())
    }

    pub fn set_rh_data(&self, rh_data: AsmRunnerRH) -> ExecutorResult<()> {
        for (_, sm) in self.sm.iter() {
            if let StateMachines::Builtin(BuiltinSMs::RomSM(rom_sm)) = sm {
                rom_sm.set_rh_data(rh_data)?;
                break;
            }
        }
        Ok(())
    }

    pub fn get_std(&self) -> Arc<Std<F>> {
        self.std.clone()
    }

    /// Routes a batch of `MemSM`-flavored plans into the planning map under
    /// the `MemSM` bucket. Used by the asm-emulator path where memory plans
    /// arrive separately from the regular planner and need to be merged.
    pub fn extend_mem_plans(&self, planning: &mut BTreeMap<usize, Vec<Plan>>, plans: Vec<Plan>) {
        planning.entry(self.mem_position).or_default().extend(plans);
    }

    pub fn plan_sec(
        &self,
        vec_counters: &mut crate::CountersChunkMetrics,
        num_chunks: usize,
    ) -> BTreeMap<usize, Vec<Plan>> {
        let mut plans = BTreeMap::new();

        for (pos, (_, sm)) in self.sm.iter().enumerate() {
            // ROM has no bus-side counter: its chunk set is just every
            // executed chunk, so plan it directly from `num_chunks`
            // instead of routing through the counters channel.
            if pos == self.rom_position {
                let rom_plan = sm_rom::RomPlanner::plan_for_chunks(num_chunks)
                    .expect("num_chunks > 0 is upheld by the caller (min_traces.len())");
                plans.insert(pos, rom_plan);
                continue;
            }

            if let Some(counters) = vec_counters.remove(&pos) {
                plans.insert(pos, sm.build_planner(self.is_asm).plan(counters));
            }
        }

        plans
    }

    pub fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &BTreeMap<usize, Vec<Plan>>) {
        for (pos, (_, sm)) in self.sm.iter().enumerate() {
            if let Some(plans) = plannings.get(&pos) {
                sm.configure_instances(pctx, plans);
            }
        }
    }

    pub fn build_instance(&self, ictx: InstanceCtx) -> ExecutorResult<Box<dyn Instance<F>>> {
        let airgroup_id = ictx.plan.airgroup_id;
        let air_id = ictx.plan.air_id;

        if airgroup_id != ZISK_AIRGROUP_ID {
            return Err(ExecutorError::StateMachineNotFound { airgroup_id, air_id });
        }

        let (_, sm) = self
            .sm
            .iter()
            .find(|(air_ids, _)| air_ids.contains(&(airgroup_id, air_id)))
            .ok_or(ExecutorError::StateMachineNotFound { airgroup_id, air_id })?;

        Ok(sm.build_instance(ictx))
    }
}
