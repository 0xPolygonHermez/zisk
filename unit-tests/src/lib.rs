//! ZisK per-AIR unit tests.
//!
//! This crate has no code of its own — it exists only to host the integration
//! tests under `tests/`. Each test uses the [`zisk_prover_backend`] test API:
//!
//! ```ignore
//! use zisk_prover_backend::{testing::with_prover, ArithEqSm, inputs::ArithEqInput};
//!
//! with_prover(|prover| {
//!     let ok = prover.verify_input().input::<ArithEqSm>(honest_input).run();
//!     assert!(ok.is_ok());
//! });
//! ```
//!
//! See `tests/` for honest-input, hook-injection, and trace-override examples.
