//! Bundle types — `StateMachines<F>` wrapper enum + `StaticSMBundle<F>` registry.
//!
//! The bundle holds every state machine the executor exposes, both
//! built-in and precompile, indexed by Vec position. Per-side details:
//!
//! * **Built-ins** — `BuiltinSMs<F>` + `BuiltinCounters` + `BuiltinCollectors` live in [`builtins`].
//! * **Precompiles** — declarative registry in [`precompiles`]; emits
//!   `Precompiles<F>` + `PrecompileCounters<F>` + `PrecompileCollectors<F>` via the
//!   `register_precompiles!` macro defined in [`register_precompiles`].
//! * **Bus construction** — `StaticDataBus::from_bundle` /
//!   `StaticDataBusCollect::for_chunk` consume a bundle to build the per-phase data buses.
//! * **Canonical entry point** — `ZiskExecutor::new` in `executor.rs`.

mod builtins;
mod precompiles;
// `register_precompiles!` macro module; exported via `#[macro_export]`.
mod register_precompiles;

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
    /// Every built-in and precompile SM registered in this bundle.
    sm: Vec<SMType<F>>,

    /// Cached Vec position of the `RomSM` entry.
    rom_position: usize,

    /// Cached Vec position of the `MemSM` entry.
    mem_position: usize,

    /// The standard library instance to be shared across built-in SMs and precompiles.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> StaticSMBundle<F> {
    /// Construct the bundle with the built-in SMs (Rom, Mem, Binary,
    /// Arith, Dma) created internally + the caller-supplied precompiles.
    pub fn new(std: Arc<Std<F>>, precompiles: Vec<(usize, Precompiles<F>)>) -> Self {
        let sm: Vec<SMType<F>> = BuiltinSMs::all(std.clone())
            .into_iter()
            .map(|(ids, b)| (ids, StateMachines::Builtin(b)))
            .chain(precompiles.into_iter().map(|(air_id, p)| {
                (
                    std::borrow::Cow::Owned(vec![(ZISK_AIRGROUP_ID, air_id)]),
                    StateMachines::Precompile(p),
                )
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

        Self { sm, rom_position, mem_position, std }
    }

    /// Read-only view of all registered SMs in insertion order.
    pub fn entries(&self) -> &[SMType<F>] {
        &self.sm
    }

    /// Sets the ROM for the `RomSM` in the bundle.
    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> ExecutorResult<()> {
        for (_, sm) in self.sm.iter() {
            if let StateMachines::Builtin(BuiltinSMs::RomSM(rom_sm)) = sm {
                rom_sm.set_rom(zisk_rom.clone())?;
            }
        }
        Ok(())
    }

    /// Sets the RH data for the `RomSM` in the bundle.
    pub fn set_rh_data(&self, rh_data: AsmRunnerRH) -> ExecutorResult<()> {
        for (_, sm) in self.sm.iter() {
            if let StateMachines::Builtin(BuiltinSMs::RomSM(rom_sm)) = sm {
                rom_sm.set_rh_data(rh_data)?;
                break;
            }
        }
        Ok(())
    }

    /// Getter for the shared `Std` instance in the bundle, used by built-in SMs and precompiles.
    pub fn get_std(&self) -> Arc<Std<F>> {
        self.std.clone()
    }

    /// Extend the plans for the `MemSM` in the bundle with the given `plans`.
    pub fn extend_mem_plans(&self, planning: &mut BTreeMap<usize, Vec<Plan>>, plans: Vec<Plan>) {
        planning.entry(self.mem_position).or_default().extend(plans);
    }

    /// Extend the plans for the secondary SMs in the bundle with the given `plans`.
    pub fn plan_sec(
        &self,
        vec_counters: &mut crate::CountersChunkMetrics,
        num_chunks: usize,
        is_asm_emulator: bool,
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
                plans.insert(pos, sm.build_planner(is_asm_emulator).plan(counters));
            }
        }

        plans
    }

    /// Configure the instances of the SMs in the bundle for the given plans.
    pub fn configure_instances(&self, pctx: &ProofCtx<F>, plannings: &BTreeMap<usize, Vec<Plan>>) {
        for (pos, (_, sm)) in self.sm.iter().enumerate() {
            if let Some(plans) = plannings.get(&pos) {
                sm.configure_instances(pctx, plans);
            }
        }
    }

    /// Builds an instance of the SM in the bundle matching the given `InstanceCtx`.
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
