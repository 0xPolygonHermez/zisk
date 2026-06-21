//! Off Linux-x86_64 (the only platform with an ASM backend) the assembly
//! runners and their shared-memory transport are `cfg`-gated out of the crate.
//! This module supplies the small slice of their surface that still has to
//! *compile* in cross-platform code elsewhere in the workspace:
//!
//! - [`AsmRunnerMO`] / [`AsmRunnerRH`] are named in `executor`'s
//!   `BackendArtifacts` and in `state-machines/rom`, and constructed (with
//!   canned data) by their unit tests. Only `new` and the public payload
//!   field are provided — `run` spawns a child process and is never reached
//!   off Linux-x86_64.
//! - [`HintsShmem`] / [`InputsShmemWriter`] are constructed by `executor`'s
//!   `AsmSharedResources` and used as `StreamSink` / `StreamProcessor` type
//!   parameters. Every method is `unreachable!`: the ASM backend is selected
//!   out at runtime here, so these are never actually driven.
#![allow(missing_docs)]

use std::sync::Arc;

use anyhow::Result;
use zisk_common::io::{StreamError, StreamProcessor, StreamSink};
use zisk_common::Plan;

use crate::{AsmRHData, AsmService, ControlShmem};

/// Runs the assembly code in a separate process to generate memory-op plans.
#[derive(Debug)]
pub struct AsmRunnerMO {
    pub plans: Vec<Plan>,
}

impl AsmRunnerMO {
    pub fn new(plans: Vec<Plan>) -> Self {
        AsmRunnerMO { plans }
    }
}

/// Runs the assembly code in a separate process to generate the ROM histogram.
pub struct AsmRunnerRH {
    pub asm_rowh_output: AsmRHData,
}

impl AsmRunnerRH {
    /// Mirrors the Linux-x86_64 signature so callers (including tests) compile
    /// uniformly. No custom `Drop` here: off Linux-x86_64 `asm_rowh_output` is
    /// an ordinary Rust-allocated `AsmRHData`, not a `Vec::from_raw_parts` view
    /// over shared memory, so it drops normally.
    pub fn new(asm_rowh_output: AsmRHData) -> Self {
        AsmRunnerRH { asm_rowh_output }
    }
}

/// Hints shared-memory sink.
pub struct HintsShmem;

impl HintsShmem {
    pub fn new(
        _shm_prefix: &str,
        _unlock_mapped_memory: bool,
        _control_writer: Arc<ControlShmem>,
        _active_services: &[AsmService],
    ) -> Result<Self> {
        unreachable!(
            "HintsShmem::new() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn bind_semaphores(&self, _sem_prefix: &str) -> Result<()> {
        unreachable!(
            "HintsShmem::bind_semaphores() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn unbind_semaphores(&self) {
        unreachable!(
            "HintsShmem::unbind_semaphores() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn set_active_services(&self, _active_services: &[AsmService]) -> Result<()> {
        unreachable!(
            "HintsShmem::set_active_services() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}

impl StreamSink for HintsShmem {
    fn submit(&self, _processed: &[u64]) -> Result<(), StreamError> {
        unreachable!(
            "HintsShmem::submit() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}

/// Inputs shared-memory writer.
pub struct InputsShmemWriter;

impl InputsShmemWriter {
    pub fn new(
        _shm_prefix: &str,
        _unlock_mapped_memory: bool,
        _control_writer: Arc<ControlShmem>,
    ) -> Result<Self> {
        unreachable!(
            "InputsShmemWriter::new() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn bind_semaphores(&self, _sem_prefix: &str) -> Result<()> {
        unreachable!(
            "InputsShmemWriter::bind_semaphores() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn unbind_semaphores(&self) {
        unreachable!(
            "InputsShmemWriter::unbind_semaphores() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn write_input(&self, _inputs: &[u8]) -> Result<()> {
        unreachable!(
            "InputsShmemWriter::write_input() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn append_input(&self, _inputs: &[u8]) -> Result<()> {
        unreachable!(
            "InputsShmemWriter::append_input() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn signal_reset(&self) -> Result<()> {
        unreachable!(
            "InputsShmemWriter::signal_reset() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn reset(&self) {
        unreachable!(
            "InputsShmemWriter::reset() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}

impl StreamProcessor for InputsShmemWriter {
    fn process_hints(&self, _data: &[u64], _first_batch: bool) -> Result<bool, StreamError> {
        unreachable!(
            "InputsShmemWriter::process_hints() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    fn reset(&self) {
        unreachable!(
            "InputsShmemWriter::reset() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}

impl StreamSink for InputsShmemWriter {
    fn submit(&self, _hints: &[u64]) -> Result<(), StreamError> {
        unreachable!(
            "InputsShmemWriter::submit() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    fn reset(&self) {
        unreachable!(
            "InputsShmemWriter::reset() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}
