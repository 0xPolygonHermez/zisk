//! Build-only crate: all the work happens in `build.rs`. Building this crate
//! (e.g. `cargo build -p zisk-definitions-sync`, or any workspace build)
//! syncs `zisk-definitions`' committed generated files with its `#[constants]`
//! definitions. Intentionally empty.
