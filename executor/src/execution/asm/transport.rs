//! [`AsmTransport`] — facade over the worker-supplied [`AsmResources`].
//!
//! The ASM emulator path needs to:
//! * receive `Arc<AsmResources>` from the worker after the C/asm
//!   children have been spawned,
//! * route stream-source / hints-submission / cancellation requests
//!   into the underlying resources,
//! * expose those resources to [`crate::EmulatorAsm`]'s execution body
//!   when it actually runs.
//!
//! `EmulatorAsm` used to hold the `RwLock<Option<Arc<AsmResources>>>`
//! and a pile of facade methods itself. This module extracts that
//! surface so:
//!   * `EmulatorAsm` is left with threading + MT-chunk logic only;
//!   * the worker can be passed `&AsmTransport` instead of the whole
//!     `EmulatorAsm`;
//!   * the lazy "resources may not be set yet" state is named in one
//!     place.
//!
//! See `.claude/executor_refactor_plan.md` step 2.2 for context.

#![cfg_attr(not(all(target_os = "linux", target_arch = "x86_64")), allow(dead_code))]

use std::sync::{Arc, RwLock};

use asm_runner::HintsShmem;
use precompiles_hints::HintsProcessor;
use zisk_common::io::StreamSource;

use crate::error::{ExecutorError, ExecutorResult};
use crate::AsmResources;

/// Wraps the optionally-set `Arc<AsmResources>` and exposes every
/// per-resource operation as a thin forwarding method.
///
/// Constructed empty (`new()`); the worker calls
/// [`Self::set_asm_resources`] once the C/asm services have been
/// spawned and the shmem segments mapped. Until then, every accessor
/// returns an "AsmResources not initialized" error.
pub struct AsmTransport {
    asm_resources: RwLock<Option<Arc<AsmResources>>>,
}

impl AsmTransport {
    /// Construct an empty transport. The worker must call
    /// [`Self::set_asm_resources`] before any of the forwarding methods
    /// can succeed.
    pub fn new() -> Self {
        Self { asm_resources: RwLock::new(None) }
    }

    /// Install the worker-supplied resources. Idempotent — calling
    /// again replaces the previously-installed value.
    pub fn set_asm_resources(&self, asm_resources: Arc<AsmResources>) -> ExecutorResult<()> {
        *self
            .asm_resources
            .write()
            .map_err(|_| ExecutorError::mutex_poisoned("asm_resources"))? = Some(asm_resources);
        Ok(())
    }

    /// Clone the currently-installed `Arc<AsmResources>`, or err if
    /// the worker hasn't installed any yet.
    ///
    /// Used by [`crate::EmulatorAsm`] when it needs the concrete
    /// `Arc<AsmResources>` to drive its execution body (spawn MO/RH,
    /// access shmem readers, etc.).
    pub fn resources(&self) -> ExecutorResult<Arc<AsmResources>> {
        self.asm_resources
            .read()
            .map_err(|_| ExecutorError::mutex_poisoned("asm_resources"))?
            .as_ref()
            .ok_or(ExecutorError::AsmResourcesNotInitialized)
            .cloned()
    }

    // ────────────────────────────────────────────────────────────
    // Forwarding methods. Each grabs the installed resources and
    // delegates; on uninstalled state, returns the standard
    // "AsmResources not initialized" error.
    // ────────────────────────────────────────────────────────────

    /// See [`AsmResources::signal_cancellation`].
    pub fn signal_cancellation(&self) -> ExecutorResult<()> {
        match self.installed_resources_ref()? {
            Some(r) => r.signal_cancellation(),
            // signal_cancellation is the only call where missing
            // resources is silently OK — the worker may call it
            // unconditionally during cleanup before resources are
            // ever installed.
            None => Ok(()),
        }
    }

    /// See [`AsmResources::get_hints_processor`].
    pub fn get_hints_processor(&self) -> ExecutorResult<Arc<HintsProcessor<HintsShmem>>> {
        self.resources()?.get_hints_processor()
    }

    /// See [`AsmResources::set_active_services`].
    pub fn set_active_services(&self, is_first_partition: bool) -> ExecutorResult<()> {
        match self.installed_resources_ref()? {
            Some(r) => r.set_active_services(is_first_partition),
            None => Ok(()),
        }
    }

    /// See [`AsmResources::set_hints_stream_src`].
    pub fn set_hints_stream_src(&self, stream: StreamSource) -> ExecutorResult<()> {
        self.resources()?.set_hints_stream_src(stream)
    }

    /// See [`AsmResources::set_inputs_stream_src`].
    pub fn set_inputs_stream_src(&self, stream: StreamSource) -> ExecutorResult<()> {
        self.resources()?.set_inputs_stream_src(stream)
    }

    /// See [`AsmResources::submit_hint_direct`].
    pub fn submit_hint_direct(&self, data: &[u64]) -> ExecutorResult<()> {
        self.resources()?.submit_hint_direct(data)
    }

    /// See [`AsmResources::append_raw_input`].
    pub fn append_raw_input(&self, bytes: &[u8]) -> ExecutorResult<()> {
        self.resources()?.append_raw_input(bytes)
    }

    /// Resets the underlying ASM pipeline (hints stream + input
    /// shmem). Mirrors [`AsmResources::reset`]. No-op when resources
    /// haven't been installed (same as the old behavior in
    /// `EmulatorAsm::reset`).
    pub fn reset(&self) -> ExecutorResult<()> {
        if let Some(r) = self.installed_resources_ref()? {
            r.reset();
        }
        Ok(())
    }

    /// Read-only borrow on the install slot, surfacing the
    /// `Option<&Arc<AsmResources>>` for methods that want to no-op
    /// when uninstalled.
    fn installed_resources_ref(&self) -> ExecutorResult<Option<Arc<AsmResources>>> {
        Ok(self
            .asm_resources
            .read()
            .map_err(|_| ExecutorError::mutex_poisoned("asm_resources"))?
            .as_ref()
            .cloned())
    }
}

impl Default for AsmTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resources_errs_before_install() {
        let t = AsmTransport::new();
        let err = t.resources().expect_err("must err when uninstalled");
        assert!(err.to_string().contains("AsmResources not initialized"));
    }

    #[test]
    fn get_hints_processor_errs_before_install() {
        // `Arc<HintsProcessor<_>>` doesn't implement `Debug`, so we
        // can't use `expect_err` — match directly.
        let t = AsmTransport::new();
        match t.get_hints_processor() {
            Ok(_) => panic!("must err when uninstalled"),
            Err(err) => assert!(err.to_string().contains("AsmResources not initialized")),
        }
    }

    #[test]
    fn signal_cancellation_silent_when_uninstalled() {
        // Cleanup paths may call this before any resources exist; it
        // must succeed silently in that case.
        let t = AsmTransport::new();
        t.signal_cancellation().expect("must be a silent no-op");
    }

    #[test]
    fn reset_silent_when_uninstalled() {
        let t = AsmTransport::new();
        t.reset().expect("must be a silent no-op");
    }

    #[test]
    fn set_active_services_silent_when_uninstalled() {
        let t = AsmTransport::new();
        t.set_active_services(true).expect("must be a silent no-op");
    }

    #[test]
    fn submit_hint_direct_errs_before_install() {
        let t = AsmTransport::new();
        let err = t.submit_hint_direct(&[]).expect_err("must err");
        assert!(err.to_string().contains("AsmResources not initialized"));
    }

    #[test]
    fn append_raw_input_errs_before_install() {
        let t = AsmTransport::new();
        let err = t.append_raw_input(&[]).expect_err("must err");
        assert!(err.to_string().contains("AsmResources not initialized"));
    }
}
