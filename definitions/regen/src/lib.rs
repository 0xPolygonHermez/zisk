//! Build-only crate: all the work happens in `build.rs`. Building this crate
//! (e.g. `cargo build -p zisk-definitions-regen`, or any workspace build)
//! regenerates `zisk-definitions`' committed C/PIL files from its `#[constants]`
//! definitions. Intentionally empty.
