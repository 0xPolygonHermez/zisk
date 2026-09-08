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

use zisk_asm_runner::AsmRunnerRH;
use zisk_common::LateJoinHandle;

use zisk_core::ZiskRom;
use zisk_sm_rom::RomSM;

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

    /// Sets the ROM for the `RomSM` in the bundle.
    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> ExecutorResult<()> {
        match self.rom_sm() {
            Some(rom_sm) => rom_sm.set_rom(zisk_rom)?,
            None => return Err(ExecutorError::BundleComponentMissing { kind: "RomSM" }),
        }
        Ok(())
    }

    /// Parks this execution's ASM ROM-histogram runner on the `RomSM` in the bundle.
    ///
    /// The runner is *not* joined here: it is read at the point of use, when the ROM
    /// instance computes its witness. Parking is what selects that instance's ASM
    /// backend, so it has to happen before the instance is built.
    ///
    /// # Errors
    /// [`ExecutorError::BundleComponentMissing`] if the bundle has no `RomSM`. Dropping
    /// the handle instead would *detach* the runner thread, leaving it reading shared
    /// memory that the next job is entitled to rewind.
    pub(crate) fn park_rh_handle(&self, handle: LateJoinHandle<AsmRunnerRH>) -> ExecutorResult<()> {
        let rom_sm =
            self.rom_sm().ok_or(ExecutorError::BundleComponentMissing { kind: "RomSM" })?;
        rom_sm.rh().park(handle);
        Ok(())
    }

    /// Publishes what this execution's ROM histogram carries for FROPS: the multiplicity
    /// column, when the assembly is where that column comes from, and the debug
    /// cross-check when it is armed.
    ///
    /// Called at the end of execution, from the executor's own thread. That is the first
    /// point the histogram is worth waiting for and the last one before its readers, all
    /// of which run after `execute` returns: the virtual tables that consume the column,
    /// and the collectors that read the cross-check flag in their constructors. Reading
    /// it here also caches it, so the ROM witness does not wait later.
    ///
    /// A no-op when neither output is wanted, so a run that uses neither never joins the
    /// runner on this thread.
    pub(crate) fn publish_frops_from_asm(&self) -> ExecutorResult<()> {
        let publish = zisk_core::frops::frops_multiplicity_from_asm();
        let cross_check = std::env::var_os(frops::CROSS_CHECK_ENV).is_some();
        if !publish && !cross_check {
            return Ok(());
        }

        // Nothing parked: the Rust emulator, or a rank that does not run the histogram.
        // Neither has a column to publish, and neither is an error.
        let Some(cell) = self.rom_sm().map(|rom_sm| rom_sm.rh()) else {
            return Ok(());
        };
        if !cell.is_armed() {
            return Ok(());
        }

        cell.with(|runner| {
            let column = &runner.asm_rowh_output.frops_count;
            if publish {
                frops::publish_frops_multiplicity(&self.std, column)?;
            }
            if cross_check {
                zisk_core::frops::load_frops_cross_check(column)
                    .map_err(ExecutorError::Internal)?;
                tracing::info!(
                    "FROPS cross-check armed from the assembly's column ({} rows)",
                    column.len()
                );
            }
            Ok(())
        })
        .map_err(|e| ExecutorError::Internal(format!("ROM histogram unavailable: {e}")))?
    }

    /// Retires a runner a previous execution left unconsumed, and releases its
    /// histogram. Must run before the next execution touches the ASM shared memory.
    pub(crate) fn drain_rh(&self) {
        if let Some(rom_sm) = self.rom_sm() {
            rom_sm.rh().drain();
        }
    }

    /// The bundle's `RomSM`, or `None` if it has none. The one place that knows where
    /// in the bundle it lives.
    fn rom_sm(&self) -> Option<&Arc<RomSM>> {
        self.sm.iter().find_map(|(_, sm)| match sm {
            StateMachines::Builtin(BuiltinSMs::RomSM(rom_sm)) => Some(rom_sm),
            _ => None,
        })
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
