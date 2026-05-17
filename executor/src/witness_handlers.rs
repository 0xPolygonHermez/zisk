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
