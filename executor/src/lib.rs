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
//!   ├── ExecutionPhase       → produces a uniform ExecutionOutput
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
//! The ASM / Rust split is encapsulated by the `EmulatorBackend` enum
//! inside `ExecutionPhase` (set once at construction, no runtime
//! dispatch). Both backends return `ExecutionOutput`; backend-specific
//! async work (the ASM MO + RH runner handles) lives in
//! `BackendArtifacts`, exposed only through `await_*` methods. **No
//! phase signature mentions `is_asm`, `JoinHandle`, or `AsmRunner*`**
//! — the backend choice is invisible past `ExecutionPhase`.
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
//! | `ExecutionPhase`           | Rust-backend construction (no ASM bring-up)   |
//! | `AsmTransport`             | Uninstalled-resource error paths              |
//! | `PlanPhase::run`           | **Integration only** — needs real `SetupCtx`  |
//! | Witness handlers           | **Integration only** — `WitnessGenerator` / `ChunkDataCollector` still take `&ProofCtx<F>` |

#![deny(missing_docs)]
#![deny(rustdoc::all)]

mod adapters;
mod bus;
mod error;
mod execution;
mod executor;
mod plan;
mod ports;
mod sm;
mod state;
mod witness;

// External API: only items re-exported here are consumed outside the
// executor crate (verified by workspace grep). Everything else is
// crate-internal.
pub use execution::asm::{AsmResources, AsmSharedResources, EmulatorAsm}; // (Linux x86_64) / stub elsewhere
pub use executor::*; // ZiskExecutor

pub(crate) use adapters::*;
pub(crate) use bus::*;
pub(crate) use execution::*;
pub(crate) use plan::*;
pub(crate) use sm::*;
pub(crate) use state::*;
pub(crate) use witness::*;

use std::collections::HashMap;

/// Type alias for chunk counters, mapping SM type ID to a list of device metrics by chunk.
pub(crate) type CountersChunkMetrics = HashMap<usize, Vec<DeviceMetricsByChunk>>;
