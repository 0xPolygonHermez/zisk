//! ZisK per-AIR unit tests.
//!
//! This crate has no code of its own — it exists only to host the integration
//! tests under `tests/`, which use the [`zisk_prover_backend`] test API
//! (`with_prover` + `verify_input().input()/.hook()/.trace_override().run()`).
//! See `tests/arith_eq.rs` for honest-input, hook-injection, and
//! trace-override examples.
