//! [`EmuExecClient`] — execute-only client backed by the Rust emulator.
//!
//! No proving keys, no `Std`, no `SetupCtx`, no `ProofMan`. Constructed
//! via [`crate::ProverClientBuilder::build_execute_only`] on the `EmuB`
//! typestate. Mirrors the existing `setup(program) → execute(stdin)`
//! pattern so per-program work (ELF parsing) is amortized across many
//! executions.

use anyhow::{Context, Result};
use executor::ZiskExecutor;
use fields::Goldilocks;
use proofman_common::VerboseMode;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_core::{Riscv2zisk, ZiskRom};

use crate::execute_client::ExecuteClient;
use crate::guest::GuestProgram;
use crate::output::ExecuteOutput;

struct EmuSetupState {
    zisk_rom: Arc<ZiskRom>,
}

/// Execute-only client (Rust emulator).
pub struct EmuExecClient {
    executor: Arc<ZiskExecutor<Goldilocks>>,
    program: Mutex<Option<EmuSetupState>>,
}

impl EmuExecClient {
    /// Construct an execute-only client. Loads no proving keys.
    pub fn new(verbose: VerboseMode) -> Result<Self> {
        let executor = ZiskExecutor::<Goldilocks>::new_standalone(verbose, false)?;
        Ok(Self { executor, program: Mutex::new(None) })
    }

    pub fn setup(&self, program: &GuestProgram) -> Result<()> {
        tracing::info!("Setting up EmuExecClient for ELF '{}'", program.name());
        tracing::debug!("Parsing ELF into ZiskRom");
        let zisk_rom = Riscv2zisk::new(program.elf())
            .run()
            .map_err(|e| anyhow::anyhow!("failed to parse ELF: {e}"))?;
        *self.program.lock().expect("program mutex") =
            Some(EmuSetupState { zisk_rom: Arc::new(zisk_rom) });
        tracing::info!("EmuExecClient ready");
        Ok(())
    }

    pub fn execute(&self, stdin: ZiskStdin) -> Result<ExecuteOutput> {
        let guard = self.program.lock().expect("program mutex");
        let setup = guard.as_ref().context("call setup(program) before execute(stdin)")?;
        tracing::info!("Running Emu execute");
        let started = Instant::now();
        let (summary, pub_outs, plan) = self.executor.execute_standalone(
            setup.zisk_rom.clone(),
            stdin,
            /* use_hints */ false,
        )?;
        Ok(ExecuteOutput::new_standalone(started.elapsed(), summary, &pub_outs, plan))
    }
}

impl ExecuteClient for EmuExecClient {
    fn setup(&self, program: &GuestProgram, _with_hints: bool) -> Result<()> {
        EmuExecClient::setup(self, program)
    }

    fn execute(
        &self,
        _program: &GuestProgram,
        stdin: ZiskStdin,
        _hints: Option<StreamSource>,
    ) -> Result<ExecuteOutput> {
        EmuExecClient::execute(self, stdin)
    }
}
