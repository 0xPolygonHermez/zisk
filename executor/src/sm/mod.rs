//! Bundle types — `StateMachines<F>` wrapper enum + `StaticSMBundle<F>` registry.
//!
//! The bundle holds every constructed state machine the executor needs at
//! **witness time** (`build_instance`, `configure_instances`, `set_rom`,
//! `park_rh_handle`). Plan-time counter/planner construction lives in this
//! module too but goes through static dispatch ([`plan_sec`], the
//! `ComponentPlanBuilder<F>` impls) and does not touch the bundle.

mod builtins;
mod frops;
mod precompiles;
// `register_precompiles!` macro module; exported via `#[macro_export]`.
mod register_precompiles;

pub use builtins::*;
pub use precompiles::*;

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::{ExecutorError, ExecutorResult};
use pil2_std_lib::Std;
use proofman_common::ProofCtx;
use proofman_fields::PrimeField64;
use zisk_common::{Instance, InstanceCtx, Plan};
use zisk_pil::ZISK_AIRGROUP_ID;

use zisk_asm_runner::{RhCell, RhJoinHandle};
use zisk_sm_rom::RomSM;

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
    pub fn new(std: Arc<Std<F>>, precompiles: Vec<(&'static [usize], Precompiles<F>)>) -> Self {
        let sm: Vec<SMType<F>> = BuiltinSMs::all(std.clone())
            .into_iter()
            .map(|(ids, b)| (ids, StateMachines::Builtin(b)))
            .chain(precompiles.into_iter().map(|(air_ids, p)| {
                // A precompile may own several airs (e.g. the ArithEq family); key it by all of them
                // so `build_instance` finds it for any of its air ids.
                (
                    std::borrow::Cow::Owned(
                        air_ids.iter().map(|&id| (ZISK_AIRGROUP_ID, id)).collect(),
                    ),
                    StateMachines::Precompile(p),
                )
            }))
            .collect();

        Self { sm, std }
    }

    /// Selects where the FROPS multiplicity column comes from.
    ///
    /// With `true` the column published by the ROM-histogram assembly is used, which only one worker
    /// computes; the collectors must then not accumulate it as well, or every multiplicity would be
    /// counted twice. With `false` (the default) the collectors own it, which is the only option on
    /// an execution path that has no ROM-histogram assembly.
    ///
    /// The choice is process state (`zisk_core::frops`) because every collector has to agree with
    /// whoever publishes the column. Set it before witness computation starts.
    pub fn set_frops_multiplicity_from_asm(&self, from_asm: bool) {
        zisk_core::frops::set_frops_multiplicity_from_asm(from_asm);
    }

    /// The bundle's ROM state machine. `BuiltinSMs::all` builds exactly one, so every
    /// ROM-specific method routes through here rather than re-walking `sm`.
    fn rom_sm(&self) -> Option<&Arc<RomSM>> {
        self.sm.iter().find_map(|(_, sm)| match sm {
            StateMachines::Builtin(BuiltinSMs::RomSM(rom_sm)) => Some(rom_sm),
            _ => None,
        })
    }

    /// The ROM-histogram handoff cell, when this bundle has a ROM state machine.
    fn rom_rh_cell(&self) -> Option<Arc<RhCell>> {
        self.rom_sm().map(|rom_sm| rom_sm.rh_cell())
    }

    /// The cell, only when this execution actually parked a runner on it. Readers that
    /// treat "no ASM histogram" as normal — the Rust emulator, a rank that does not run
    /// RH — go through here so `NotArmed` stays an error for genuine handoff failures.
    fn armed_rh_cell(&self) -> Option<Arc<RhCell>> {
        self.rom_rh_cell().filter(|cell| cell.is_armed())
    }

    /// Sets the ROM for the `RomSM` in the bundle.
    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> ExecutorResult<()> {
        match self.rom_sm() {
            Some(rom_sm) => Ok(rom_sm.set_rom(zisk_rom)?),
            None => Ok(()),
        }
    }

    /// Parks this execution's ASM ROM-histogram runner on the `RomSM` in the bundle.
    ///
    /// The runner is *not* joined here. Everything that needs the histogram's contents
    /// reads it through the cell later, at the end of the execution — see
    /// [`Self::finish_rh`].
    pub(crate) fn park_rh_handle(&self, handle: RhJoinHandle) {
        if let Some(cell) = self.rom_rh_cell() {
            cell.park(handle);
        }
    }

    /// Runs everything that reads this execution's ROM histogram, in the one order that
    /// works: join it first, then serve the readers from the cached value.
    ///
    /// Called at the end of execution, from the executor's own thread, because the join is
    /// the part that can block and the readers that come afterwards run on proofman
    /// witness threads, which hold core permits out of a bounded pool while they wait.
    pub(crate) fn finish_rh(&self) -> ExecutorResult<()> {
        self.resolve_rh()?;
        self.publish_frops_from_asm()?;
        self.arm_frops_cross_check()
    }

    /// Joins this execution's ROM-histogram runner and caches the result, so the ROM
    /// witness does not have to wait for it later. A no-op when nothing is parked.
    fn resolve_rh(&self) -> ExecutorResult<()> {
        match self.armed_rh_cell() {
            Some(cell) => Ok(cell.resolve()?),
            None => Ok(()),
        }
    }

    /// Publishes the FROPS multiplicity column that the ROM-histogram assembly built, when
    /// that is where the column comes from (see [`Self::set_frops_multiplicity_from_asm`]);
    /// a no-op otherwise.
    ///
    /// Called once per execution, at the end of execution — the same phase the column used
    /// to be published from, and well before the virtual tables that consume it, which are
    /// computed after every non-table instance.
    fn publish_frops_from_asm(&self) -> ExecutorResult<()> {
        if !zisk_core::frops::frops_multiplicity_from_asm() {
            return Ok(());
        }
        // The column comes from the assembly, so a bundle without the state machine that
        // holds it cannot produce one.
        let cell =
            self.rom_rh_cell().ok_or(ExecutorError::BundleComponentMissing { kind: "RomSM" })?;
        cell.with_histogram(|rh| frops::publish_frops_multiplicity(&self.std, &rh.frops_count))?
    }

    /// Arms the debug cross-check of the two producers of the FROPS column, when
    /// `ZISK_FROPS_CROSS_CHECK` is set; a no-op otherwise.
    ///
    /// Must run before the first collector is built — they read whether the check is armed
    /// in their constructors — which the end of execution still is: collectors are built
    /// from `pre_calculate` / `collect_single`, both after `execute` returns.
    fn arm_frops_cross_check(&self) -> ExecutorResult<()> {
        if std::env::var_os(frops::CROSS_CHECK_ENV).is_none() {
            return Ok(());
        }
        // No assembly histogram this run (Rust emulator, or a rank that does not run RH):
        // there is no column to cross-check against.
        let Some(cell) = self.armed_rh_cell() else {
            return Ok(());
        };
        cell.with_histogram(|rh| {
            zisk_core::frops::load_frops_cross_check(&rh.frops_count)
                .map_err(ExecutorError::Internal)?;
            tracing::info!(
                "FROPS cross-check armed from the assembly's column ({} rows)",
                rh.frops_count.len()
            );
            Ok(())
        })?
    }

    /// Drains a runner the previous execution left unconsumed, and releases its
    /// histogram. Must run before the next execution touches the ASM shared memory.
    pub(crate) fn drain_rh(&self) {
        if let Some(cell) = self.rom_rh_cell() {
            cell.drain();
        }
    }

    /// Getter for the shared `Std` instance in the bundle, used by built-in SMs and precompiles.
    pub fn get_std(&self) -> Arc<Std<F>> {
        self.std.clone()
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
    let rom_plan = zisk_sm_rom::RomPlanner::plan_for_chunks(num_chunks)
        .expect("num_chunks > 0 is upheld by the caller (min_traces.len())");
    plans.insert(ROM_POSITION, rom_plan);

    for pos in [MEM_POSITION, BINARY_POSITION, ARITH_POSITION, DMA_POSITION, JUMP_DEST_POSITION] {
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
