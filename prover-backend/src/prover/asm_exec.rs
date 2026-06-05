//! [`AsmExecClient`] — execute-only client backed by the ASM emulator.
//!
//! No proving keys, no `Std`, no `SetupCtx`, no `ProofMan`. Constructed
//! via [`crate::ProverClientBuilder::build_execute_only`] on the `AsmB`
//! typestate. Mirrors the existing `setup(program) → execute(stdin)`
//! pattern so per-program work (ELF parsing, ASM binary generation, ASM
//! subprocess spawn) is amortized across many executions.

use anyhow::{Context, Result};
use executor::{AsmResources, ZiskExecutor};
use fields::Goldilocks;
use proofman_common::VerboseMode;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_core::{Riscv2zisk, ZiskRom};

use crate::execute_client::ExecuteClient;
use crate::guest::GuestProgram;
use crate::output::ExecuteOutput;

struct AsmSetupState {
    zisk_rom: Arc<ZiskRom>,
    with_hints: bool,
}

pub struct AsmExecClient {
    executor: Arc<ZiskExecutor<Goldilocks>>,
    asm_cache_dir: PathBuf,
    verbose: VerboseMode,
    program: Mutex<Option<AsmSetupState>>,
}

impl AsmExecClient {
    pub fn new(verbose: VerboseMode, asm_cache_dir: Option<PathBuf>) -> Result<Self> {
        let asm_cache_dir = rom_setup::get_output_path(&asm_cache_dir)?;
        let executor = ZiskExecutor::<Goldilocks>::new_standalone(verbose, true)?;
        Ok(Self { executor, asm_cache_dir, verbose, program: Mutex::new(None) })
    }

    pub fn setup(&self, program: &GuestProgram, with_hints: bool) -> Result<()> {
        tracing::info!(
            "Setting up AsmExecClient for ELF '{}' (with_hints={})",
            program.name(),
            with_hints
        );
        let mt_path = self.ensure_asm_binaries(program, with_hints)?;

        tracing::debug!("Spawning ASM services and mapping shmem");
        let resources = Arc::new(
            AsmResources::new_standalone(
                program.hash().to_string(),
                &mt_path,
                with_hints,
                self.verbose,
            )
            .context("AsmResources::new_standalone failed")?,
        );
        self.executor.set_asm_resources(resources)?;

        tracing::debug!("Parsing ELF into ZiskRom");
        let zisk_rom = Riscv2zisk::new(program.elf())
            .run()
            .map_err(|e| anyhow::anyhow!("failed to parse ELF: {e}"))?;
        *self.program.lock().expect("program mutex") =
            Some(AsmSetupState { zisk_rom: Arc::new(zisk_rom), with_hints });
        tracing::info!("AsmExecClient ready");
        Ok(())
    }

    pub fn execute(&self, stdin: ZiskStdin, hints: Option<StreamSource>) -> Result<ExecuteOutput> {
        let guard = self.program.lock().expect("program mutex");
        let setup = guard.as_ref().context("call setup(program, with_hints) before execute")?;

        if let Some(stream) = hints {
            tracing::debug!("Installing hints stream source");
            if let Some(asm) = self.executor.asm_emulator() {
                asm.set_hints_stream_src(stream).context(
                    "set_hints_stream_src failed (was setup called with with_hints=true?)",
                )?;
            }
        }

        tracing::info!("Running ASM execute (with_hints={})", setup.with_hints);
        let started = Instant::now();
        let (summary, pub_outs, plan) =
            self.executor.execute_standalone(setup.zisk_rom.clone(), stdin, setup.with_hints)?;
        Ok(ExecuteOutput::new_standalone(started.elapsed(), summary, &pub_outs, plan))
    }

    /// Resolves cached `<base>-mt.bin` path. Generates all 3 ASM binaries
    /// via `rom_setup::generate_assembly` if any are missing.
    fn ensure_asm_binaries(&self, program: &GuestProgram, with_hints: bool) -> Result<PathBuf> {
        let [mt, rh, mo] = rom_setup::get_assembly_file_paths_from_id(
            program.name(),
            program.hash(),
            &self.asm_cache_dir,
            with_hints,
        );

        if mt.exists() && rh.exists() && mo.exists() {
            tracing::debug!(
                "Using cached ASM binaries for ELF '{}' at {}",
                program.name(),
                self.asm_cache_dir.display()
            );
            return Ok(mt);
        }

        tracing::info!(
            "Generating ASM binaries for ELF '{}' (one-time per ELF) at {} (with_hints={})",
            program.name(),
            self.asm_cache_dir.display(),
            with_hints
        );
        let gen_verbose = matches!(self.verbose, VerboseMode::Debug | VerboseMode::Trace);
        rom_setup::generate_assembly(
            program.elf(),
            program.name(),
            &self.asm_cache_dir,
            with_hints,
            gen_verbose,
        )
        .context("rom_setup::generate_assembly failed")?;
        tracing::info!("ASM binaries generated for ELF '{}'", program.name());
        Ok(mt)
    }
}

impl ExecuteClient for AsmExecClient {
    fn setup(&self, program: &GuestProgram, with_hints: bool) -> Result<()> {
        AsmExecClient::setup(self, program, with_hints)
    }

    fn execute(
        &self,
        _program: &GuestProgram,
        stdin: ZiskStdin,
        hints: Option<StreamSource>,
    ) -> Result<ExecuteOutput> {
        AsmExecClient::execute(self, stdin, hints)
    }
}
