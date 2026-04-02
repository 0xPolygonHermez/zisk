use std::path::PathBuf;

use crate::cancel::CancellationToken;
use crate::ZiskStdin;
use anyhow::Result;
use proofman_common::ParamsGPU;
use zisk_common::ProofMode;
use zisk_common::{ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};
use zisk_prover_backend::{
    get_proving_key, get_proving_key_snark, Asm, AsmProver, Emu, EmuProver, GuestProgram,
    ProofOpts, ProverEngine, ZiskProver,
};

use crate::{execute::ExecuteResult, input::ProgramInput, proof::Proof, Client, ExecutorKind};

const ERR_ASSEMBLY_NOT_ENABLED: &str =
    "Assembly executor not enabled — call .assembly() on the builder";

/// Configuration for the embedded prover backend.
#[derive(Default)]
pub struct EmbeddedClientConfig {
    pub(crate) proving_key: Option<PathBuf>,
    pub(crate) proving_key_snark: Option<PathBuf>,
}

/// Builder for an embedded [`ProverClient`].
pub(crate) struct EmbeddedClientBuilder {
    executor: ExecutorKind,
    gpu_params: Option<ParamsGPU>,
    config: EmbeddedClientConfig,
}

impl EmbeddedClientBuilder {
    pub(crate) fn new(config: EmbeddedClientConfig) -> Self {
        Self { executor: ExecutorKind::Emulator, gpu_params: None, config }
    }

    #[must_use]
    pub(crate) fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = executor;
        self
    }

    #[must_use]
    pub(crate) fn with_gpu_params(mut self, gpu_params: ParamsGPU) -> Self {
        self.gpu_params = Some(gpu_params);
        self
    }

    pub(crate) fn build(self) -> Result<EmbeddedClient> {
        let pk = get_proving_key(self.config.proving_key.as_ref());
        let pk_snark = get_proving_key_snark(self.config.proving_key_snark.as_ref());
        let prover = match self.executor {
            ExecutorKind::Emulator => Self::build_emu(pk, pk_snark, self.gpu_params)?,
            ExecutorKind::Assembly => Self::build_asm(pk, pk_snark, self.gpu_params)?,
        };
        Ok(EmbeddedClient { prover })
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
        Ok(EmbeddedProver::Emu(ZiskProver::<Emu>::new(emu)))
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
        Ok(EmbeddedProver::Asm(ZiskProver::<Asm>::new(asm)))
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
        cancel: Option<&CancellationToken>,
    ) -> Result<Proof> {
        // Check for cancellation before starting.
        if cancel.map_or(false, |t| t.is_cancelled()) {
            anyhow::bail!("Operation was cancelled");
        }
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
                apply_mode!(p.prove(program, stdin.into_inner()).with_proof_options(opts)).run()?
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
                apply_mode!(p.prove(program, stdin.into_inner()).with_proof_options(opts)).run()?
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Hints(hints)) => {
                if !p.was_setup_with_hints() {
                    anyhow::bail!(
                        "Program was set up without hints — pass ZiskStdin, not ZiskHints"
                    );
                }
                p.register_hints_stream(hints.into_inner())?;
                apply_mode!(p
                    .prove(program, ZiskStdin::null().into_inner())
                    .with_proof_options(opts))
                .run()?
            }
        };
        Ok(Proof::new(result))
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        input: ProgramInput,
        executor: ExecutorKind,
        cancel: Option<&CancellationToken>,
    ) -> Result<ExecuteResult> {
        // Check for cancellation before starting.
        if cancel.map_or(false, |t| t.is_cancelled()) {
            anyhow::bail!("Operation was cancelled");
        }
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
                if p.was_setup_with_hints() {
                    anyhow::bail!("Program was set up with hints — pass ZiskHints, not ZiskStdin");
                }
                p.execute(program, stdin.into_inner())?
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Hints(hints)) => {
                if !p.was_setup_with_hints() {
                    anyhow::bail!(
                        "Program was set up without hints — pass ZiskStdin, not ZiskHints"
                    );
                }
                p.register_hints_stream(hints.into_inner())?;
                p.execute(program, ZiskStdin::null().into_inner())?
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
                p.prover.reduce(&proof_with_publics.proof, publics, program_vk)
            }
            EmbeddedProver::Asm(p) => {
                p.prover.reduce(&proof_with_publics.proof, publics, program_vk)
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
                p.prover.plonk(&proof_with_publics.proof, publics, program_vk)
            }
            EmbeddedProver::Asm(p) => {
                p.prover.plonk(&proof_with_publics.proof, publics, program_vk)
            }
        }
    }
}
