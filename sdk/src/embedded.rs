use std::path::PathBuf;

use crate::cancel::CancellationToken;
use crate::ZiskStdin;
use anyhow::Result;
use proofman_common::ProofmanOptions;
use zisk_common::ProofMode;
use zisk_common::{ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};
use zisk_prover_backend::{
    get_packed_info, get_proving_key, get_proving_key_snark, Asm, AsmProver, Emu, EmuProver,
    GuestProgram, ProofOpts, ProverEngine, ZiskProver,
};

use crate::{
    execute::ExecuteResult, input::ProgramInput, proof::Proof, Client, ExecutorKind, ProofKind,
};

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
    proof_kind: ProofKind,
    options: ProofmanOptions,
    config: EmbeddedClientConfig,
}

impl EmbeddedClientBuilder {
    pub(crate) fn new(config: EmbeddedClientConfig) -> Self {
        Self {
            executor: ExecutorKind::Emulator,
            proof_kind: ProofKind::StarkMinimal,
            options: ProofmanOptions::default(),
            config,
        }
    }

    #[must_use]
    pub(crate) fn executor(mut self, executor: ExecutorKind) -> Self {
        self.executor = executor;
        self
    }

    #[must_use]
    pub(crate) fn gpu(mut self) -> Self {
        self.options.gpu();
        self.options.packed_info(get_packed_info());
        self
    }

    #[must_use]
    pub(crate) fn plonk(mut self) -> Self {
        self.proof_kind = ProofKind::Plonk;
        self
    }

    pub(crate) fn build(self) -> Result<EmbeddedClient> {
        let pk = get_proving_key(self.config.proving_key.as_ref());
        let pk_snark = get_proving_key_snark(self.config.proving_key_snark.as_ref());
        let use_snark = matches!(self.proof_kind, ProofKind::Plonk);
        let prover = match self.executor {
            ExecutorKind::Emulator => Self::build_emu(pk, pk_snark, self.options, use_snark)?,
            ExecutorKind::Assembly => Self::build_asm(pk, pk_snark, self.options, use_snark)?,
        };
        Ok(EmbeddedClient { prover })
    }

    fn build_emu(
        pk: PathBuf,
        pk_snark: PathBuf,
        options: ProofmanOptions,
        use_snark: bool,
    ) -> Result<EmbeddedProver> {
        let emu = EmuProver::new(use_snark, false, pk, pk_snark, false, options, None)?;
        Ok(EmbeddedProver::Emu(ZiskProver::<Emu>::new(emu)))
    }

    fn build_asm(
        pk: PathBuf,
        pk_snark: PathBuf,
        options: ProofmanOptions,
        use_snark: bool,
    ) -> Result<EmbeddedProver> {
        let asm = AsmProver::new(
            use_snark, false, pk, pk_snark, false, None, false, false, false, options, false, None,
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
        if cancel.is_some_and(|t| t.is_cancelled()) {
            anyhow::bail!("Operation was cancelled");
        }
        macro_rules! apply_mode {
            ($builder:expr) => {
                match mode {
                    ProofMode::VadcopFinal => $builder,
                    ProofMode::VadcopFinalMinimal => $builder.minimal(),
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
                if p.was_setup_with_hints()? {
                    anyhow::bail!("Program was set up with hints — pass ZiskHints, not ZiskStdin");
                }
                apply_mode!(p.prove(program, stdin.into_inner()).with_proof_options(opts)).run()?
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly, ProgramInput::Hints(hints)) => {
                if !p.was_setup_with_hints()? {
                    anyhow::bail!(
                        "Program was set up without hints — pass ZiskStdin, not ZiskHints"
                    );
                }
                p.register_hints_stream(hints.into_inner())?;
                apply_mode!(p
                    .prove(program, ZiskStdin::new().into_inner())
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
        if cancel.is_some_and(|t| t.is_cancelled()) {
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

    fn run_minimal(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        let publics = override_publics.unwrap_or(&proof_with_publics.publics);
        let program_vk = override_program_vk.unwrap_or(&proof_with_publics.program_vk);
        match &self.prover {
            EmbeddedProver::Emu(p) => {
                p.prover.minimal(&proof_with_publics.proof, publics, program_vk)
            }
            EmbeddedProver::Asm(p) => {
                p.prover.minimal(&proof_with_publics.proof, publics, program_vk)
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
