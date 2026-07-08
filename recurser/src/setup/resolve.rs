//! Re-export pil2-proofman's path-resolution helpers. Do NOT copy them locally:
//! they bake `CARGO_MANIFEST_DIR` at compile time, so a copy here would resolve
//! relative to the zisk crate instead of pil2-proofman (breaking the fallback).

pub(super) use pil2_stark_setup::commands::recursive_setup::{
    resolve_circom_exec, resolve_path_env,
};
