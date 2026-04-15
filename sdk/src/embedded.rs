pub(crate) mod execute;
pub(crate) mod prove;
pub(crate) mod setup;
pub(crate) mod upload;
pub(crate) mod wrap;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::setup::SetupResult;
use crate::ZiskStdin;
use anyhow::Result;
use zisk_common::ProofMode;
use zisk_common::{ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};
use zisk_prover_backend::{
    get_proving_key, get_proving_key_snark, Asm, AsmOptions, AsmProver, Emu, EmuProver,
    GuestProgram, ProverEngine, ZiskProver,
};

use crate::{
    execute::{ExecuteRequest, ExecuteResult},
    input::ProgramInput,
    job_handle::{fire_event, fire_result_event, JobHandle, JobHandleInner, SubscriberList},
    opts::ProverOpts,
    proof::Proof,
    prove::{JobEvent, ProveRequest},
    setup::SetupRequest,
    upload::UploadRequest,
    wrap::WrapRequest,
    Client, ExecutorKind, ProofKind,
};

const ERR_ASSEMBLY_NOT_ENABLED: &str =
    "Assembly executor not enabled — call .assembly() on the builder";

/// Builder for an embedded [`ProverClient`].
pub struct EmbeddedClientBuilder {
    executor: ExecutorKind,
    proof_kind: ProofKind,
    prover_options: ProverOpts,
    gpu: bool,
    asm_options: Option<AsmOptions>,
    proving_key: Option<PathBuf>,
    proving_key_snark: Option<PathBuf>,
}

impl Default for EmbeddedClientBuilder {
    fn default() -> Self {
        Self {
            executor: ExecutorKind::Emulator,
            proof_kind: ProofKind::StarkMinimal,
            prover_options: ProverOpts::default(),
            gpu: false,
            asm_options: None,
            proving_key: None,
            proving_key_snark: None,
        }
    }
}

impl EmbeddedClientBuilder {
    /// Set the executor kind. Default is [`ExecutorKind::Emulator`].
    #[must_use]
    pub fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = executor;
        self
    }

    /// Use the Emulator executor (default). Not compatible with hints.
    #[must_use]
    pub fn emulator(mut self) -> Self {
        self.executor = ExecutorKind::Emulator;
        self
    }

    /// Use the Assembly executor.
    #[must_use]
    pub fn assembly(mut self) -> Self {
        self.executor = ExecutorKind::Assembly;
        self
    }

    /// Set proof generation options (e.g. minimal memory mode).
    #[must_use]
    pub fn with_prover_options(mut self, opts: ProverOpts) -> Self {
        self.prover_options = opts;
        self
    }

    /// Enable GPU acceleration.
    #[must_use]
    pub fn gpu(mut self) -> Self {
        self.gpu = true;
        self
    }

    /// Enable PLONK proof mode.
    #[must_use]
    pub fn plonk(mut self) -> Self {
        self.proof_kind = ProofKind::Plonk;
        self
    }

    /// Set ASM-specific options. Only valid with the Assembly executor.
    #[must_use]
    pub fn asm_options(mut self, opts: AsmOptions) -> Self {
        self.asm_options = Some(opts);
        self
    }

    /// Set the path to the proving key directory.
    #[must_use]
    pub fn proving_key(mut self, path: impl Into<PathBuf>) -> Self {
        self.proving_key = Some(path.into());
        self
    }

    /// Set the path to the PLONK proving key directory.
    #[must_use]
    pub fn proving_key_plonk(mut self, path: impl Into<PathBuf>) -> Self {
        self.proving_key_snark = Some(path.into());
        self
    }

    /// Build the [`EmbeddedClient`].
    pub fn build(self) -> Result<EmbeddedClient> {
        crate::client::ensure_single_instance();
        if self.asm_options.is_some() && self.executor != ExecutorKind::Assembly {
            panic!(
                "asm_options were set but the executor is not Assembly. \
                 Call .assembly() on the builder before setting asm_options."
            );
        }
        let mut prover_options = self.prover_options;
        if let Some(pk) = self.proving_key {
            prover_options.proving_key = Some(pk);
        }
        if let Some(pk) = self.proving_key_snark {
            prover_options.proving_key_snark = Some(pk);
        }
        let mut backend_opts = prover_options.into_backend_opts(self.gpu);
        if let Some(asm_opts) = self.asm_options {
            *backend_opts.asm_options_mut() = asm_opts;
        }
        let pk = get_proving_key(backend_opts.get_proving_key());
        let pk_snark = get_proving_key_snark(backend_opts.get_proving_key_snark());
        let prover = match self.executor {
            ExecutorKind::Emulator => Self::build_emu(pk, pk_snark, backend_opts, self.proof_kind)?,
            ExecutorKind::Assembly => Self::build_asm(pk, pk_snark, backend_opts, self.proof_kind)?,
        };
        Ok(EmbeddedClient { prover: Arc::new(prover) })
    }

    fn build_emu(
        pk: PathBuf,
        pk_snark: PathBuf,
        backend_opts: zisk_prover_backend::BackendProverOpts,
        proof_kind: ProofKind,
    ) -> Result<EmbeddedProver> {
        let emu = EmuProver::new(
            proof_kind == ProofKind::Plonk,        // plonk
            backend_opts.preload_plonk(),          // preload_snark
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
        let asm_opts = backend_opts.asm_options();
        let asm = AsmProver::new(
            proof_kind == ProofKind::Plonk,        // plonk
            backend_opts.preload_plonk(),          // preload_snark
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

pub struct EmbeddedClient {
    prover: Arc<EmbeddedProver>,
}

impl Clone for EmbeddedClient {
    fn clone(&self) -> Self {
        Self { prover: Arc::clone(&self.prover) }
    }
}

impl EmbeddedClient {
    pub(crate) fn run_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
    ) -> Result<SetupResult> {
        match self.prover.as_ref() {
            EmbeddedProver::Emu(p) => {
                p.setup(program).run()?;
                Ok(SetupResult)
            }
            EmbeddedProver::Asm(p) => {
                let builder = p.setup(program);
                if with_hints {
                    builder.with_hints().run()?;
                } else {
                    builder.run()?;
                }
                Ok(SetupResult)
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
        let result = match (self.prover.as_ref(), executor, input) {
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
        let result = match (self.prover.as_ref(), executor, input) {
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
        match self.prover.as_ref() {
            EmbeddedProver::Emu(p) => {
                p.prover.wrap_proof(&proof_with_publics.proof, publics, program_vk, mode)
            }
            EmbeddedProver::Asm(p) => {
                p.prover.wrap_proof(&proof_with_publics.proof, publics, program_vk, mode)
            }
        }
    }
}

impl Client for EmbeddedClient {
    fn run_upload(&self, program: &GuestProgram) -> Result<crate::upload::UploadResult> {
        upload::run(self, program)
    }

    fn run_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        setup::run(self.clone(), program, with_hints, timeout, subs)
    }

    fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        mode: ProofMode,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<Proof>> {
        prove::run(self.clone(), program, input, executor, mode, timeout, subs)
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ExecuteResult>> {
        execute::run(self.clone(), program, input, executor, timeout, subs)
    }

    fn run_wrap(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        mode: ProofMode,
        override_publics: Option<ZiskPublics>,
        override_program_vk: Option<ZiskProgramVK>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ZiskProofWithPublicValues>> {
        wrap::run(
            self.clone(),
            proof_with_publics,
            mode,
            override_publics,
            override_program_vk,
            timeout,
            subs,
        )
    }
}

impl EmbeddedClient {
    /// Submit a prove request.
    #[must_use]
    pub fn prove<'a>(
        &'a self,
        program: &'a GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> ProveRequest<'a, Self> {
        ProveRequest::new(self, program, input)
    }

    /// Submit an execute request (dry-run, no proof).
    #[must_use]
    pub fn execute<'a>(
        &'a self,
        program: &'a GuestProgram,
        input: impl Into<ProgramInput>,
    ) -> ExecuteRequest<'a, Self> {
        ExecuteRequest::new(self, program, input)
    }

    /// Submit a ROM setup request.
    #[must_use]
    pub fn setup<'a>(&'a self, program: &'a GuestProgram) -> SetupRequest<'a, Self> {
        SetupRequest::new(self, program)
    }

    /// Submit an upload request (no-op for embedded — program is available locally).
    #[must_use]
    pub fn upload<'a>(&'a self, program: &'a GuestProgram) -> UploadRequest<'a, Self> {
        UploadRequest::new(self, program)
    }

    /// Submit a wrap/convert proof request.
    #[must_use]
    pub fn wrap_proof<'a>(
        &'a self,
        proof_with_publics: &'a ZiskProofWithPublicValues,
        mode: ProofMode,
    ) -> WrapRequest<'a, Self> {
        WrapRequest::new(self, proof_with_publics, mode)
    }
}

impl Default for EmbeddedClient {
    fn default() -> Self {
        EmbeddedClientBuilder::default()
            .build()
            .expect("Failed to initialize default EmbeddedClient")
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
