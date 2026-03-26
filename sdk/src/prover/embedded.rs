use std::path::PathBuf;

use anyhow::Result;
use proofman_common::ParamsGPU;
use zisk_common::io::ZiskStdin;
use zisk_common::ZiskProgramVK;
use zisk_prover_backend::{
    get_proving_key, get_proving_key_snark, Asm, AsmProver, Emu, EmuProver, GuestProgram,
    ProofOpts, ZiskProver,
};

use crate::{
    client::ProverClient, core::public_prover::PublicZiskProver, execute::ExecuteResult,
    proof::Proof, Client, ExecutorKind,
};

const ERR_ASSEMBLY_NOT_ENABLED: &str =
    "Assembly executor not enabled — call .assembly() on the builder";

/// Configuration for an embedded prover backend.
#[derive(Default)]
pub struct EmbeddedOptions {
    pub proving_key: Option<PathBuf>,
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

    /// Enable Assembly executor support. Emulator remains the per-request default.
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
        let pk_snark = get_proving_key_snark(None);
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
        Ok(EmbeddedProver::Emu(PublicZiskProver::new(ZiskProver::<Emu>::new(emu))))
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
        Ok(EmbeddedProver::Asm(PublicZiskProver::new(ZiskProver::<Asm>::new(asm))))
    }
}

enum EmbeddedProver {
    Emu(PublicZiskProver<Emu>),
    Asm(PublicZiskProver<Asm>),
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
    fn run_prove(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        executor: ExecutorKind,
        opts: ProofOpts,
    ) -> Result<Proof> {
        let result = match (&self.prover, executor) {
            (EmbeddedProver::Emu(p), ExecutorKind::Emulator) => {
                p.prove(program, stdin).with_proof_options(opts).run()?
            }
            (EmbeddedProver::Emu(_), ExecutorKind::Assembly) => {
                anyhow::bail!(ERR_ASSEMBLY_NOT_ENABLED)
            }
            (EmbeddedProver::Asm(_p), ExecutorKind::Emulator) => {
                unimplemented!("Assembly prover does not yet support emulation mode"); // TODO: implement prove_emu()
                // p.prove(program, stdin).with_proof_options(opts).run()? // TODO: replace with prove_emu()
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly) => {
                p.prove(program, stdin).with_proof_options(opts).run()?
            }
        };
        Ok(Proof::new(result))
    }

    fn run_execute(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        executor: ExecutorKind,
    ) -> Result<ExecuteResult> {
        let result = match (&self.prover, executor) {
            (EmbeddedProver::Emu(p), ExecutorKind::Emulator) => p.execute(program, stdin)?,
            (EmbeddedProver::Emu(_), ExecutorKind::Assembly) => {
                anyhow::bail!(ERR_ASSEMBLY_NOT_ENABLED)
            }
            (EmbeddedProver::Asm(_p), ExecutorKind::Emulator) => {
                unimplemented!("Assembly prover does not yet support emulation mode"); // TODO: implement execute_emu()
                // p.execute(program, stdin)? // TODO: replace with execute_emu()
            }
            (EmbeddedProver::Asm(p), ExecutorKind::Assembly) => p.execute(program, stdin)?,
        };
        Ok(ExecuteResult::new(result))
    }
}
