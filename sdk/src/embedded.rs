use std::path::PathBuf;

use crate::ZiskStdin;
use anyhow::Result;
use proofman_common::ParamsGPU;
use zisk_common::ProofMode;
use zisk_common::{ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};
use zisk_prover_backend::{
    get_proving_key, get_proving_key_snark, Asm, AsmProver, Emu, EmuProver, GuestProgram,
    ProofOpts, ProverEngine, ZiskProver,
};

use crate::{
    client::ProverClient, core::sdk_prover::ZiskProverSDK, execute::ExecuteResult,
    input::ProgramInput, proof::Proof, Client, ExecutorKind,
};

const ERR_ASSEMBLY_NOT_ENABLED: &str =
    "Assembly executor not enabled — call .assembly() on the builder";

/// Configuration for an embedded prover backend.
#[derive(Default)]
pub struct EmbeddedOptions {
    proving_key: Option<PathBuf>,
    proving_key_snark: Option<PathBuf>,
}

impl EmbeddedOptions {
    /// Set the path to the proving key directory.
    #[must_use]
    pub fn proving_key(mut self, path: impl Into<PathBuf>) -> Self {
        self.proving_key = Some(path.into());
        self
    }

    /// Set the path to the SNARK proving key directory.
    #[must_use]
    pub fn proving_key_snark(mut self, path: impl Into<PathBuf>) -> Self {
        self.proving_key_snark = Some(path.into());
        self
    }
}

/// Builder for an embedded [`ProverClient`].
///
/// Obtain via [`ProverClient::embedded`].
pub struct EmbeddedClientBuilder {
    executor: ExecutorKind,
    gpu_params: Option<ParamsGPU>,
    options: EmbeddedOptions,
}

impl EmbeddedClientBuilder {
    pub(crate) fn new(options: EmbeddedOptions) -> Self {
        Self { executor: ExecutorKind::Emulator, gpu_params: None, options }
    }

    /// Enable a specific executor. Default is `ExecutorKind::Emulator`.
    #[must_use]
    pub fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = executor;
        self
    }

    /// Enable `ExecutorKind::Emulator` executor (default). Not compatible with hints.
    #[must_use]
    pub fn emulator(mut self) -> Self {
        self.executor = ExecutorKind::Emulator;
        self
    }

    /// Enable `ExecutorKind::Assembly` executor.
    #[must_use]
    pub fn assembly(mut self) -> Self {
        self.executor = ExecutorKind::Assembly;
        self
    }

    /// Enable GPU acceleration with default parameters.
    #[must_use]
    pub fn gpu(mut self) -> Self {
        self.gpu_params = Some(ParamsGPU::default());
        self
    }

    /// Enable GPU acceleration with custom parameters.
    #[must_use]
    pub fn with_gpu_params(mut self, gpu_params: ParamsGPU) -> Self {
        self.gpu_params = Some(gpu_params);
        self
    }

    pub fn build(self) -> Result<ProverClient> {
        let pk = get_proving_key(self.options.proving_key.as_ref());
        let pk_snark = get_proving_key_snark(self.options.proving_key_snark.as_ref());
        let prover = match self.executor {
            ExecutorKind::Emulator => Self::build_emu(pk, pk_snark, self.gpu_params)?,
            ExecutorKind::Assembly => Self::build_asm(pk, pk_snark, self.gpu_params)?,
        };
        Ok(ProverClient::from_embedded(EmbeddedClient { prover }))
    }

    fn build_emu(
        pk: PathBuf,
        pk_snark: PathBuf,
        gpu_params: Option<ParamsGPU>,
    ) -> Result<EmbeddedProver> {
        let emu = EmuProver::new(
            false,
            true,
            false,
            pk,
            pk_snark,
            0,
            false,
            gpu_params.unwrap_or_default(),
            None,
        )?;
        Ok(EmbeddedProver::Emu(ZiskProverSDK::new(ZiskProver::<Emu>::new(emu))))
    }

    fn build_asm(
        pk: PathBuf,
        pk_snark: PathBuf,
        gpu_params: Option<ParamsGPU>,
    ) -> Result<EmbeddedProver> {
        let asm = AsmProver::new(
            false,
            true,
            false,
            pk,
            pk_snark,
            0,
            false,
            None,
            false,
            false,
            false,
            gpu_params.unwrap_or_default(),
            false,
            None,
        )?;
        Ok(EmbeddedProver::Asm(ZiskProverSDK::new(ZiskProver::<Asm>::new(asm))))
    }
}

enum EmbeddedProver {
    Emu(ZiskProverSDK<Emu>),
    Asm(ZiskProverSDK<Asm>),
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

    pub(crate) fn cancel(&self) {
        match &self.prover {
            EmbeddedProver::Emu(p) => p.inner.cancel(),
            EmbeddedProver::Asm(p) => p.inner.cancel(),
        }
    }
}

impl Client for EmbeddedClient {
    fn run_upload(&self, _program: &GuestProgram) -> Result<()> {
        // No upload step needed for embedded client since it has direct access to the ELF files.
        Ok(())
    }

    fn run_setup(&self, program: &GuestProgram, with_hints: bool) -> Result<()> {
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

    fn run_prove(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        mode: ProofMode,
        opts: ProofOpts,
    ) -> Result<Proof> {
        macro_rules! apply_mode {
            ($builder:expr) => {
                match mode {
                    ProofMode::VadcopFinal => $builder,
                    ProofMode::VadcopFinalReduced => $builder.reduced(),
                    ProofMode::Snark => $builder.plonk(),
                }
            };
        }
        let result = match (&self.prover, executor, input) {
            (EmbeddedProver::Emu(p), ExecutorKind::Emulator, ProgramInput::Stdin(stdin)) => {
                apply_mode!(p.prove(program, stdin).with_proof_options(opts)).run()?
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
                if p.was_setup_with_hints() {
                    anyhow::bail!("Program was set up with hints — pass ZiskHints, not ZiskStdin");
                }
                apply_mode!(p.prove(program, stdin).with_proof_options(opts)).run()?
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Hints(hints)) => {
                if !p.was_setup_with_hints() {
                    anyhow::bail!(
                        "Program was set up without hints — pass ZiskStdin, not ZiskHints"
                    );
                }
                p.register_hints_stream(hints.into_inner())?;
                apply_mode!(p.prove(program, ZiskStdin::null()).with_proof_options(opts)).run()?
            }
        };
        Ok(Proof::new(result))
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
    ) -> Result<ExecuteResult> {
        let result = match (&self.prover, executor, input) {
            (EmbeddedProver::Emu(p), ExecutorKind::Emulator, ProgramInput::Stdin(stdin)) => {
                p.execute(program, stdin)?
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
                if p.was_setup_with_hints() {
                    anyhow::bail!("Program was set up with hints — pass ZiskHints, not ZiskStdin");
                }
                p.execute(program, stdin)?
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Hints(hints)) => {
                if !p.was_setup_with_hints() {
                    anyhow::bail!(
                        "Program was set up without hints — pass ZiskStdin, not ZiskHints"
                    );
                }
                p.register_hints_stream(hints.into_inner())?;
                p.execute(program, ZiskStdin::null())?
            }
        };
        Ok(ExecuteResult::new(result))
    }

    fn run_reduce(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        let publics = override_publics.unwrap_or(&proof_with_publics.publics);
        let program_vk = override_program_vk.unwrap_or(&proof_with_publics.program_vk);
        match &self.prover {
            EmbeddedProver::Emu(p) => {
                p.inner.prover.reduce(&proof_with_publics.proof, publics, program_vk)
            }
            EmbeddedProver::Asm(p) => {
                p.inner.prover.reduce(&proof_with_publics.proof, publics, program_vk)
            }
        }
    }

    fn run_plonk(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        let publics = override_publics.unwrap_or(&proof_with_publics.publics);
        let program_vk = override_program_vk.unwrap_or(&proof_with_publics.program_vk);
        match &self.prover {
            EmbeddedProver::Emu(p) => {
                p.inner.prover.plonk(&proof_with_publics.proof, publics, program_vk)
            }
            EmbeddedProver::Asm(p) => {
                p.inner.prover.plonk(&proof_with_publics.proof, publics, program_vk)
            }
        }
    }
}
