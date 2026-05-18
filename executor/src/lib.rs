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
//!   ├── TracePhase           → produces a uniform ExecutionOutput
//!   │     (chooses EmulatorAsm or EmulatorRust at construction)
//!   │
//!   └── PlanPhase            → consumes the ExecutionOutput
//!         │  (plan + materialize; one phase actor)
//!         ├── plan_main           (pure, unit-testable)
//!         ├── plan_secondary      (drains counters into per-SM plans)
//!         ├── InstancePlanner     (ROM/main/secn global-id assignment)
//!         ├── InstanceRegistry / InstanceFactory
//!         │     → fills InstanceSet + checkpoints
//!         └── returns PlanOutput
//!               (instance data + timings + cost_per_type)
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
//! The ASM / Rust split lives behind the `Emulator<F>` trait. Both
//! impls return `ExecutionOutput`; backend-specific async work (the
//! ASM MO + RH runner handles) is encapsulated in `BackendArtifacts`,
//! exposed only through `await_*` methods. **No phase signature
//! mentions `is_asm`, `JoinHandle`, or `AsmRunner*`** — the backend
//! choice is invisible past `TracePhase`.
//!
//! # Anti-corruption layer (ACL)
//!
//! `ProofCtx<F>` is consumed inside the executor through two port
//! traits:
//!
//! * `Dctx` — instance info, rank ownership, witness-ready flag.
//!   Used by the witness handlers via trait object.
//! * `ProofRegistry` — extends `Dctx` with `add_instance*` /
//!   `add_table` / `find_instance_id` / `set_chunks` / `write_pub_outs`,
//!   used by `PlanPhase`.
//!
//! `ProofmanAdapter` is the concrete adapter wrapping `&ProofCtx<F>`.
//!
//! # Testability
//!
//! | Component                  | Test surface                                  |
//! |----------------------------|-----------------------------------------------|
//! | `BackendArtifacts`         | Synthetic `JoinHandle`s, fake threads         |
//! | `PlanPhase::plan_main`     | Synthetic `EmuTrace` array (no `ProofCtx`)    |
//! | `InstancePlanner`          | `FakeProofRegistry` records call sequences    |
//! | `AsmRunnerSupervisor`      | Fake `JoinHandle`s, failure-path tests        |
//! | `MtChunkProcessor`         | Synthetic chunk plumbing                      |
//! | `InstanceSet` / `ChunkCollectorStore` | Construct + reset / is_empty       |
//! | `TracePhase`               | Rust-backend construction (no ASM bring-up)   |
//! | `AsmTransport`             | Uninstalled-resource error paths              |
//! | `PlanPhase::run`           | **Integration only** — needs real `SetupCtx`  |
//! | Witness handlers           | **Integration only** — `WitnessGenerator` / `ChunkDataCollector` still take `&ProofCtx<F>` |

#![deny(missing_docs)]
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

// External API: only these 4 items are consumed outside the executor
// crate (verified by workspace grep). Everything else is crate-internal.
pub use asm_resources::*; // AsmResources, AsmSharedResources
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use emu_asm::*; // EmulatorAsm
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use emu_asm_stub::*;
pub use executor::*; // ZiskExecutor

use adapters::*;
use air_classifier::*;
use asm_runner_supervisor::*;
use asm_transport::*;
use chunk_collector_store::*;
use collector::*;
use dummy_counter::*;
use emu_rust::*;
use init::*;
use instance_factory::*;
use instance_set::*;
use mt_chunk_processor::*;
use plan_phase::*;
use planner::*;
use registry::*;
use sm_builtins::*;
use sm_precompiles::*;
use state::*;
use static_data_bus::*;
use static_data_bus_collect::*;
use trace_output::*;
use trace_phase::*;
use witness_generator::*;
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
/// Both backends return a uniform `ExecutionOutput`; backend-specific
/// async work (ASM-only MO + RH handles) is encapsulated in
/// `ExecutionOutput::backend` and exposed via the `await_*` methods on
/// `BackendArtifacts`.
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
    ) -> Result<ExecutionOutput>;
}
