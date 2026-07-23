//! ZisK constant groups for multi-target codegen.
//!
//! [`ZISK_CONSTANTS`] is the table the `zisk-definitions-sync` build renders to
//! `src/generated/`. It is deliberately empty ("void") today: the real ZisK
//! constants migrate in here **one module per file**, and a group only starts
//! producing generated output once it is listed in `ZISK_CONSTANTS`.
//!
//! The pattern each group file follows is shown by the sample groups
//! ([`memory`]/[`opcodes`]/[`execution`], compiled under `test` only): an inline
//! `#[constants]` module plus a `pub use <group>::{GROUP, EXPORTS};` re-export. The
//! `tests` round-trip renders them to exercise every attribute feature: inheritance,
//! `#[emit(internal)]`, `skip(..)`, target restriction, derived values, a per-target
//! prefix, a radix override, and a `fits` override.

// The emission schema, used to type `ZISK_CONSTANTS`. The `#[constants]` macro
// references `zisk_definitions_generator::meta::*` directly, so no re-export is needed.
use zisk_definitions_generator::meta;

/// Groups rendered to `src/generated/` by the sync build. Add a group here once its
/// module is real (promote it out of `#[cfg(test)]` and list it below).
pub const ZISK_CONSTANTS: &[(&meta::GroupMeta, &[meta::Export])] =
    &[(&execution::GROUP, execution::EXPORTS)];

mod execution;

// Sample groups — the shape a real group takes, one module per file. Compiled under
// `test` only until a group is real and wired into `ZISK_CONSTANTS` above.
#[cfg(test)]
mod memory;
#[cfg(test)]
mod opcodes;

#[cfg(test)]
mod tests;
