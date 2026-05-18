//! Per-category witness-compute handlers, dispatched by
//! [`crate::WitnessRouter`].
//!
//! Each module owns the witness-compute path for one air-id category:
//! main, secondary (non-ROM `Instance`), ROM under native backend,
//! ROM under ASM backend, table. Shared helpers
//! (`take_collectors_for_instance`, `register_empty_collector`) live
//! in `common`.
//!
//! Step 4.3 of the executor refactor: the previous monolithic
//! `WitnessRouter::compute_secondary_witness` is now five focused
//! handlers, each in its own file and reviewable in isolation.

pub(crate) mod common;
pub mod main;
pub mod rom_asm;
pub mod rom_native;
pub mod secondary;
pub mod table;

pub use main::MainWitnessHandler;
pub use rom_asm::RomAsmWitnessHandler;
pub use rom_native::RomNativeWitnessHandler;
pub use secondary::SecondaryWitnessHandler;
pub use table::TableWitnessHandler;

use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::Result;
use fields::PrimeField64;
use proofman_common::{ProofCtx, SetupCtx};
use zisk_common::Instance;

use crate::ports::Dctx;
use crate::state::ExecutionState;
use crate::{ChunkDataCollector, WitnessGenerator};

/// Map of secondary instances keyed by `global_id`. Used by the ROM
/// pre-calculate path to fetch a specific instance.
pub(crate) type SecnInstanceMap<F> = HashMap<usize, Box<dyn Instance<F>>>;

/// Map of borrowed secondary instances, populated by the ROM/secondary
/// pre-calculate paths and consumed by `ChunkDataCollector::collect`.
pub(crate) type SecnInstanceMapRef<'a, F> = HashMap<usize, &'a Box<dyn Instance<F>>>;

/// Strategy interface for the ROM witness path.
///
/// `WitnessRouter` holds one boxed implementor — chosen at construction
/// via `new_asm` / `new_native` — so both the dispatch and pre-calculate
/// paths route through the same instance without any per-call backend
/// branching. The two impls live in [`rom_asm`] and [`rom_native`].
pub(crate) trait RomWitnessHandler<F: PrimeField64>: Send + Sync {
    /// Compute the witness for a ROM global id. Takes the union of the
    /// two backends' arguments — the ASM impl ignores `collector`; the
    /// native impl ignores `airgroup_id` / `air_id`.
    #[allow(clippy::too_many_arguments)]
    fn dispatch(
        &self,
        generator: &WitnessGenerator,
        collector: &ChunkDataCollector<F>,
        trace_buffer_rom: &Mutex<Vec<F>>,
        state: &ExecutionState<F>,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_id: usize,
        airgroup_id: usize,
        air_id: usize,
        stats_scope_id: u64,
    ) -> Result<()>;

    /// Pre-calculate hook for ROM ids. ASM marks the gid not-ready
    /// unconditionally; native may also register an empty collector and
    /// flip the gid to ready (when `RomInstance::skip_collector()` is
    /// true), or enqueue the instance into `instances_to_collect`.
    #[allow(clippy::too_many_arguments)]
    fn pre_calculate<'a>(
        &self,
        registry: &dyn Dctx,
        state: &ExecutionState<F>,
        secn_instances: &'a SecnInstanceMap<F>,
        instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
        global_id: usize,
        airgroup_id: usize,
        air_id: usize,
    ) -> Result<()>;
}
