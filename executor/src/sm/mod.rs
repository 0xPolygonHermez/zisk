//! Bundle types — `StateMachines<F>` wrapper enum + `StaticSMBundle<F>` registry.
//!
//! The bundle holds every constructed state machine the executor needs at
//! **witness time** (`build_instance`, `configure_instances`, `set_rom`,
//! `set_rh_data`). Plan-time counter/planner construction lives in this
//! module too but goes through static dispatch ([`plan_sec`], the
//! `ComponentPlanBuilder<F>` impls) and does not touch the bundle.

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
use zisk_common::{Instance, InstanceCtx, Plan};
use zisk_pil::ZISK_AIRGROUP_ID;

use asm_runner::AsmRunnerRH;

use zisk_core::ZiskRom;

pub type SMType<F> = (SMAirType, StateMachines<F>);

pub enum StateMachines<F: PrimeField64> {
    Builtin(BuiltinSMs<F>),
    Precompile(Precompiles<F>),
}

impl<F: PrimeField64> StateMachines<F> {
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

        Self { sm, std }
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

    /// All registered state machines (built-ins + precompiles), each paired
    /// with the AIR ids it serves. Used by the unit-test executor to build
    /// its AIR-id → inner-SM manager registry.
    pub fn iter_sms(&self) -> impl Iterator<Item = &SMType<F>> {
        self.sm.iter()
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

/// Plans secondary instances via static dispatch. Builtins use position
/// constants; precompiles iterate `PRECOMPILE_AIR_IDS` and dispatch by
/// air id. Drains `vec_counters` via `remove`.
pub fn plan_sec<F: PrimeField64>(
    vec_counters: &mut crate::CountersChunkMetrics,
    num_chunks: usize,
    is_asm_emulator: bool,
) -> BTreeMap<usize, Vec<Plan>> {
    let mut plans = BTreeMap::new();

    // ROM has no bus-side counter — plan from chunk count directly.
    let rom_plan = sm_rom::RomPlanner::plan_for_chunks(num_chunks)
        .expect("num_chunks > 0 is upheld by the caller (min_traces.len())");
    plans.insert(ROM_POSITION, rom_plan);

    for pos in [MEM_POSITION, BINARY_POSITION, ARITH_POSITION, DMA_POSITION] {
        if let Some(counters) = vec_counters.remove(&pos) {
            let planner = BuiltinSMs::<F>::planner_for_position(pos, is_asm_emulator);
            plans.insert(pos, planner.plan(counters));
        }
    }

    for (i, &air_id) in PRECOMPILE_AIR_IDS.iter().enumerate() {
        let pos = BUILTIN_COUNT + i;
        if let Some(counters) = vec_counters.remove(&pos) {
            let planner = Precompiles::<F>::planner_for_air_id(air_id, is_asm_emulator);
            plans.insert(pos, planner.plan(counters));
        }
    }

    plans
}

/// Appends mem-related plans (from the ASM MO runner) into the mem slot.
pub fn extend_mem_plans(planning: &mut BTreeMap<usize, Vec<Plan>>, plans: Vec<Plan>) {
    planning.entry(MEM_POSITION).or_default().extend(plans);
}
