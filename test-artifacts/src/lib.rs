//! Compiles every program under `programs/` into an ELF and re-exports each one
//! as a constant for use by other crates in the workspace.
//!
//! **Adding a new program:** register it as a member of the `programs/` workspace
//! so it gets built, then expose its ELF via a `load_program!` constant below.

use zisk_sdk::{load_program, GuestProgram};

pub const ELF_BLAKE2: GuestProgram = load_program!("blake2");
pub const ELF_DIAGNOSTIC: GuestProgram = load_program!("diagnostic");
