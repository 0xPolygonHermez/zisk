pub(crate) mod execute;
pub(crate) mod prove;
pub(crate) mod setup;
pub(crate) mod upload;
pub(crate) mod wrap;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::ZiskStdin;
use anyhow::Result;
use zisk_common::ProofMode;
use zisk_common::{ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};
use zisk_prover_backend::{
    get_proving_key, get_proving_key_snark, Asm, AsmOptions, AsmProver, Emu, EmuProver,
    GuestProgram, ProverEngine, ZiskProver,
};

use crate::{
    execute::ExecuteResult,
    input::ProgramInput,
    job_handle::{fire_event, fire_result_event, JobHandle, JobHandleInner, SubscriberList},
    opts::ProverOpts,
    proof::Proof,
    prove::JobEvent,
    ExecutorKind, ProofKind,
};

const ERR_ASSEMBLY_NOT_ENABLED: &str =
    "Assembly executor not enabled — call .assembly() on the builder";

/// Configuration for embedded (local) prover client.
#[derive(Clone, Default)]
pub struct EmbeddedClientConfig {
    pub proving_key: Option<PathBuf>,
    pub proving_key_snark: Option<PathBuf>,
}

/// Builder for an embedded [`ProverClient`].
pub(crate) struct EmbeddedClientBuilder {
    executor: ExecutorKind,
    proof_kind: ProofKind,
    prover_options: ProverOpts,
    gpu: bool,
    asm_options: Option<AsmOptions>,
}

impl EmbeddedClientBuilder {
    pub(crate) fn new(config: EmbeddedClientConfig) -> Self {
        let mut prover_options = ProverOpts::default();
        if let Some(pk) = config.proving_key {
            prover_options.proving_key = Some(pk);
        }
        if let Some(pk) = config.proving_key_snark {
            prover_options.proving_key_snark = Some(pk);
        }
        Self {
            executor: ExecutorKind::Emulator,
            proof_kind: ProofKind::StarkMinimal,
            prover_options,
            gpu: false,
            asm_options: None,
        }
    }

    #[must_use]
    pub(crate) fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = executor;
        self
    }

    #[must_use]
    pub(crate) fn with_prover_options(mut self, opts: ProverOpts) -> Self {
        self.prover_options = opts;
        self
    }

    #[must_use]
    pub(crate) fn gpu(mut self) -> Self {
        self.gpu = true;
        self
    }

    #[must_use]
    pub(crate) fn plonk(mut self) -> Self {
        self.proof_kind = ProofKind::Plonk;
        self
    }

    #[must_use]
    pub(crate) fn asm_options(mut self, opts: AsmOptions) -> Self {
        self.asm_options = Some(opts);
        self
    }

    pub(crate) fn build(self) -> Result<EmbeddedClient> {
        let mut backend_opts = self.prover_options.into_backend_opts(self.gpu);
        if let Some(asm_opts) = self.asm_options {
            *backend_opts.get_asm_options_mut() = asm_opts;
        }
        let pk = get_proving_key(backend_opts.get_proving_key());
        let pk_snark = get_proving_key_snark(backend_opts.get_proving_key_snark());
        let prover = match self.executor {
            ExecutorKind::Emulator => Self::build_emu(pk, pk_snark, backend_opts, self.proof_kind)?,
            ExecutorKind::Assembly => Self::build_asm(pk, pk_snark, backend_opts, self.proof_kind)?,
        };
        Ok(EmbeddedClient { prover })
    }

    fn build_emu(
        pk: PathBuf,
        pk_snark: PathBuf,
        backend_opts: zisk_prover_backend::BackendProverOpts,
        proof_kind: ProofKind,
    ) -> Result<EmbeddedProver> {
        let emu = EmuProver::new(
            proof_kind == ProofKind::Plonk,        // plonk
            backend_opts.get_preload_plonk(),      // preload_snark
            pk,                                    // proving_key
            pk_snark,                              // proving_key_snark
            true,                                  // shared_tables
            backend_opts.build_proofman_options(), // options
            None,                                  // logging_config
        )?;
        Ok(EmbeddedProver::Emu(ZiskProver::<Emu>::new(emu, backend_opts)))
    }

    fn build_asm(
        pk: PathBuf,
        pk_snark: PathBuf,
        backend_opts: zisk_prover_backend::BackendProverOpts,
        proof_kind: ProofKind,
    ) -> Result<EmbeddedProver> {
        let asm_opts = backend_opts.get_asm_options();
        let asm = AsmProver::new(
            proof_kind == ProofKind::Plonk,        // plonk
            backend_opts.get_preload_plonk(),      // preload_snark
            pk,                                    // proving_key
            pk_snark,                              // proving_key_snark
            true,                                  // shared_tables
            asm_opts.base_port,                    // base_port
            asm_opts.unlock_mapped_memory,         // unlock_mapped_memory
            asm_opts.asm_out_file,                 // asm_out_file
            asm_opts.no_auto_setup,                // no_auto_setup
            backend_opts.build_proofman_options(), // options
            false,                                 // is_distributed
            None,                                  // logging_config
        )?;
        Ok(EmbeddedProver::Asm(ZiskProver::<Asm>::new(asm, backend_opts)))
    }
}

enum EmbeddedProver {
    Emu(ZiskProver<Emu>),
    Asm(ZiskProver<Asm>),
}

pub(crate) struct EmbeddedClient {
    prover: EmbeddedProver,
}

impl EmbeddedClient {
    pub(crate) fn vk(&self, program: &GuestProgram) -> Result<ZiskProgramVK> {
        match &self.prover {
            EmbeddedProver::Emu(p) => p.vk(program),
            EmbeddedProver::Asm(p) => p.vk(program),
        }
    }

    pub(crate) fn run_setup(&self, program: &GuestProgram, with_hints: bool) -> Result<()> {
        match &self.prover {
            EmbeddedProver::Emu(p) => p.setup(program).run(),
            EmbeddedProver::Asm(p) => {
                let builder = p.setup(program);
                if with_hints {
                    builder.with_hints().run()
                } else {
                    builder.run()
                }
            }
        }
    }

    pub(crate) fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        mode: ProofMode,
    ) -> Result<Proof> {
        macro_rules! apply_mode {
            ($builder:expr) => {
                match mode {
                    ProofMode::VadcopFinal => $builder,
                    ProofMode::VadcopFinalMinimal => {
                        $builder.wrap_proof(ProofMode::VadcopFinalMinimal)
                    }
                    ProofMode::Plonk => $builder.wrap_proof(ProofMode::Plonk),
                }
            };
        }
        let result = match (&self.prover, executor, input) {
            (EmbeddedProver::Emu(p), ExecutorKind::Emulator, ProgramInput::Stdin(stdin)) => {
                apply_mode!(p.prove(program, stdin.into_inner())).run()?
            }
            (EmbeddedProver::Emu(_), ExecutorKind::Emulator, ProgramInput::Hints(_)) => {
                anyhow::bail!("Hints require Assembly executor")
            }
            (EmbeddedProver::Emu(_), ExecutorKind::Assembly, _) => {
                anyhow::bail!(ERR_ASSEMBLY_NOT_ENABLED)
            }
            (EmbeddedProver::Asm(_), ExecutorKind::Emulator, _) => {
                unimplemented!("Assembly prover does not yet support emulation mode")
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Stdin(stdin)) => {
                if p.was_setup_with_hints()? {
                    anyhow::bail!("Program was set up with hints — pass ZiskHints, not ZiskStdin");
                }
                apply_mode!(p.prove(program, stdin.into_inner())).run()?
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Hints(hints)) => {
                if !p.was_setup_with_hints()? {
                    anyhow::bail!(
                        "Program was set up without hints — pass ZiskStdin, not ZiskHints"
                    );
                }
                p.register_hints_stream(hints.into_inner())?;
                apply_mode!(p.prove(program, ZiskStdin::new().into_inner())).run()?
            }
        };
        Ok(Proof::new(result))
    }

    pub(crate) fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
    ) -> Result<ExecuteResult> {
        let result = match (&self.prover, executor, input) {
            (EmbeddedProver::Emu(p), ExecutorKind::Emulator, ProgramInput::Stdin(stdin)) => {
                p.execute(program, stdin.into_inner())?
            }
            (EmbeddedProver::Emu(_), ExecutorKind::Emulator, ProgramInput::Hints(_)) => {
                anyhow::bail!("Hints require Assembly executor")
            }
            (EmbeddedProver::Emu(_), ExecutorKind::Assembly, _) => {
                anyhow::bail!(ERR_ASSEMBLY_NOT_ENABLED)
            }
            (EmbeddedProver::Asm(_), ExecutorKind::Emulator, _) => {
                unimplemented!("Assembly prover does not yet support emulation mode")
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Stdin(stdin)) => {
                if p.was_setup_with_hints()? {
                    anyhow::bail!("Program was set up with hints — pass ZiskHints, not ZiskStdin");
                }
                p.execute(program, stdin.into_inner())?
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Hints(hints)) => {
                if !p.was_setup_with_hints()? {
                    anyhow::bail!(
                        "Program was set up without hints — pass ZiskStdin, not ZiskHints"
                    );
                }
                p.register_hints_stream(hints.into_inner())?;
                p.execute(program, ZiskStdin::new().into_inner())?
            }
        };
        Ok(ExecuteResult::new(result))
    }

    pub(crate) fn run_wrap(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        mode: ProofMode,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        let publics = override_publics.unwrap_or(&proof_with_publics.publics);
        let program_vk = override_program_vk.unwrap_or(&proof_with_publics.program_vk);
        match &self.prover {
            EmbeddedProver::Emu(p) => {
                p.prover.wrap_proof(&proof_with_publics.proof, publics, program_vk, mode)
            }
            EmbeddedProver::Asm(p) => {
                p.prover.wrap_proof(&proof_with_publics.proof, publics, program_vk, mode)
            }
        }
    }
}

/// Spawn a blocking embedded job, firing Started/Completed/Failed events around `f`.
pub(crate) fn spawn_embedded_job<T, F>(
    f: F,
    timeout: Option<Duration>,
    subs: SubscriberList,
) -> Result<JobHandle<T>>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    let subs_task = Arc::clone(&subs);
    let handle = tokio::task::spawn_blocking(move || {
        fire_event(&subs_task, JobEvent::Started);
        let result = f();
        fire_result_event(&subs_task, &result);
        result
    });
    Ok(JobHandle { inner: JobHandleInner::Embedded(handle), subscribers: subs, timeout })
}
