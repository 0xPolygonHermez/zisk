//! Executor crate: runs ZisK ROMs across multiple emulator backends and
//! drives every step needed to hand a populated proof context to the
//! witness side.
//!
//! # Architecture overview
//!
//! The executor pipeline is split into four phase actors, each owning a
//! distinct responsibility. Data flows downstream as typed phase
//! outputs:
//!
//! ```text
//! ZiskExecutor::execute
//!   │
//!   ├── TracePhase           → produces a uniform TraceOutput
//!   │     (chooses EmulatorAsm or EmulatorRust at construction)
//!   │
//!   └── MaterializePhase     → consumes the TraceOutput
//!         │  (replaces phases 2–4 of the old monolithic execute)
//!         ├── PlanPhase (called internally)
//!         │     ├── plan_main  (pure, unit-testable)
//!         │     └── plan_secondary (drains counters into per-SM plans)
//!         │
//!         ├── InstancePlanner (ROM/main/secn global-id assignment)
//!         ├── InstanceRegistry / InstanceFactory
//!         │     → fills InstanceSet + checkpoints
//!         └── returns MaterializeOutput
//!               (timings + cost_per_type)
//!
//! ZiskExecutor::calculate_witness
//!   │
//!   └── WitnessRouter::dispatch
//!         │  (router has its backend baked at construction via
//!         │   `new_asm` / `new_native`)
//!         ├── MainWitnessHandler
//!         ├── SecondaryWitnessHandler
//!         ├── RomNativeWitnessHandler
//!         ├── RomAsmWitnessHandler
//!         └── TableWitnessHandler
//! ```
//!
//! # Backend abstraction
//!
//! The ASM / Rust split lives behind the [`Emulator<F>`] trait. Both
//! impls return [`TraceOutput`]; backend-specific async work (the
//! ASM MO + RH runner handles) is encapsulated in [`BackendArtifacts`],
//! exposed only through `await_*` methods. **No phase signature
//! mentions `is_asm`, `JoinHandle`, or `AsmRunner*`** — the backend
//! choice is invisible past `TracePhase`.
//!
//! # Anti-corruption layer (ACL)
//!
//! `ProofCtx<F>` is consumed inside the executor through three port
//! traits:
//!
//! * [`Dctx`] — instance info, rank ownership, witness-ready flag.
//! * [`ProofRegistry`] — extends [`Dctx`] with `add_instance*` /
//!   `add_table` / `find_instance_id` / `set_chunks` / `write_pub_outs`,
//!   used by `MaterializePhase`.
//! * [`WitnessRegistry<F>`] — extends [`Dctx`] with `add_air_instance`,
//!   used by the witness handlers.
//!
//! [`ProofmanAdapter`] is the concrete adapter wrapping `&ProofCtx<F>`.
//!
//! # Testability
//!
//! After the M0–M4 refactor:
//!
//! | Component                  | Test surface                                  |
//! |----------------------------|-----------------------------------------------|
//! | `AirClassifier`            | Pure functions (no setup)                     |
//! | `PubOutsCollector`         | Pure functions (no setup)                     |
//! | [`BackendArtifacts`]       | Synthetic `JoinHandle`s, fake threads         |
//! | [`PlanPhase`] `::plan_main`| Synthetic `EmuTrace` array (no `ProofCtx`)    |
//! | `InstancePlanner`          | `FakeProofRegistry` records call sequences    |
//! | [`AsmRunnerSupervisor`]    | Fake `JoinHandle`s, failure-path tests        |
//! | [`MtChunkProcessor`]       | Synthetic chunk plumbing                      |
//! | [`InstanceSet`] / [`ChunkCollectorStore`] | Construct + reset / is_empty   |
//! | [`TracePhase`]             | Rust-backend construction (no ASM bring-up)   |
//! | [`AsmTransport`]           | Uninstalled-resource error paths              |
//! | [`MaterializePhase`]       | **Integration only** — needs real `ProofCtx`  |
//! | Witness handlers           | **Integration only** — `WitnessGenerator` / `ChunkDataCollector` still take `&ProofCtx<F>` |
//!
//! Adding unit-level coverage to the integration-only rows means
//! pushing the ACL through `WitnessGenerator` + `ChunkDataCollector`
//! (which currently take `&ProofCtx<F>` directly) — a deferred
//! follow-up, not blocking.

#![warn(missing_docs)] // ratchet up to deny once clean
#![warn(rustdoc::all)] // broken intra-doc links, invalid HTML, bare URLs
#![deny(rustdoc::all)]

mod adapters;
mod air_classifier;
mod asm_resources;
mod asm_runner_supervisor;
mod asm_transport;
mod chunk_collector_store;
mod collector;
mod dummy_counter;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod emu_asm;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod emu_asm_stub;
mod emu_rust;
mod executor;
mod init;
mod instance_factory;
mod instance_set;
mod materialize_phase;
mod mt_chunk_processor;
mod plan_phase;
mod planner;
mod ports;
mod pub_outs_collector;
mod registry;
mod sm_builtins;
mod sm_precompiles;
mod sm_registry;
mod state;
mod static_data_bus;
mod static_data_bus_collect;
mod trace_output;
mod trace_phase;

mod witness_generator;
mod witness_handlers;
mod witness_router;

pub use adapters::*;
use air_classifier::*;
pub use asm_resources::*;
pub use asm_runner_supervisor::*;
pub use asm_transport::*;
pub use chunk_collector_store::*;
use collector::*;
pub use dummy_counter::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use emu_asm::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use emu_asm_stub::*;
pub use emu_rust::*;
pub use executor::*;
pub use init::*;
pub use instance_factory::*;
pub use instance_set::*;
pub use materialize_phase::*;
pub use mt_chunk_processor::*;
pub use plan_phase::*;
use planner::*;
pub use ports::*;
use registry::*;
pub use sm_builtins::*;
pub use sm_precompiles::*;
pub use state::*;
pub use static_data_bus::*;
pub use static_data_bus_collect::*;
pub use trace_output::*;
pub use trace_phase::*;
use witness_generator::*;
pub use witness_handlers::*;
use witness_router::*;
use zisk_core::ZiskRom;
/// Type alias for chunk counters, mapping SM type ID to a list of device metrics by chunk.
pub type CountersChunkMetrics = HashMap<usize, Vec<DeviceMetricsByChunk>>;

use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::collections::HashMap;
use zisk_common::{io::ZiskStdin, ExecutorStatsHandle, StatsScope};

use anyhow::Result;

/// Trait for unified execution across different emulator backends.
///
/// Both backends return a uniform [`TraceOutput`]; backend-specific
/// async work (ASM-only MO + RH handles) is encapsulated in
/// [`TraceOutput::backend`] and exposed via the `await_*` methods on
/// [`BackendArtifacts`].
#[allow(clippy::too_many_arguments)]
pub trait Emulator<F: PrimeField64>: Send + Sync {
    /// Execute the emulator
    fn execute(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &ZiskStdin,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> Result<TraceOutput>;
}
