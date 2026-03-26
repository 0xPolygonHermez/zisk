use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use proofman_common::ParamsGPU;
use zisk_prover_backend::{
    get_proving_key, get_proving_key_snark, Asm, AsmProver, Emu, EmuProver, ZiskProver,
};

use super::execute::ExecuteRequest;
use super::proof::Proof;
use super::prove::ProveRequest;
use super::setup::SetupRequest;
use super::types::{ClientConfig, Executor};
use super::upload::UploadRequest;
use crate::core::public_prover::PublicZiskProver;
use crate::GuestProgram;
use zisk_common::io::ZiskStdin;
use zisk_common::ZiskProgramVK;
use zisk_prover_backend::ProofOpts;

static PROVER_CLIENT_CREATED: AtomicBool = AtomicBool::new(false);

fn ensure_single_instance() -> Result<()> {
    if PROVER_CLIENT_CREATED.swap(true, Ordering::AcqRel) {
        anyhow::bail!(
            "A ProverClient already exists. Only one instance is allowed per process. \
             Store it in a shared location (e.g., Arc<ProverClient>) and reuse it."
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// EmbeddedOptions
// ---------------------------------------------------------------------------

/// Configuration for an embedded prover backend.
#[derive(Default)]
pub struct EmbeddedOptions {
    pub proving_key: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// RemoteOptions
// ---------------------------------------------------------------------------

// /// Configuration for a remote prover backend.
// pub struct RemoteOptions {
//     pub url: String,
//     pub api_key: Option<String>,
// }

// impl RemoteOptions {
//     pub fn builder() -> RemoteOptionsBuilder {
//         RemoteOptionsBuilder::default()
//     }
// }

// /// Builder for `RemoteOptions`.
// #[derive(Default)]
// pub struct RemoteOptionsBuilder {
//     url: Option<String>,
//     api_key: Option<String>,
// }

// impl RemoteOptionsBuilder {
//     #[must_use]
//     pub fn url(mut self, url: impl Into<String>) -> Self {
//         self.url = Some(url.into());
//         self
//     }

//     #[must_use]
//     pub fn api_key(mut self, key: impl Into<String>) -> Self {
//         self.api_key = Some(key.into());
//         self
//     }

//     pub fn build(self) -> Result<RemoteOptions> {
//         let url = self.url.ok_or_else(|| anyhow::anyhow!("RemoteOptions requires a URL"))?;
//         Ok(RemoteOptions { url, api_key: self.api_key })
//     }
// }

// ---------------------------------------------------------------------------
// EmbeddedProver — runtime backend dispatch
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub enum ProverKind {
    Emu(PublicZiskProver<Emu>),
    Asm(PublicZiskProver<Asm>),
}

// ---------------------------------------------------------------------------
// ProverClient — embedded backend
// ---------------------------------------------------------------------------

/// Embedded prover client. Runs proofs using local infrastructure.
///
/// Obtain via:
/// - `ProverClient::default()` — zero-config (Emulator, no GPU)
/// - `ProverClient::embedded(opts).build()` — full configuration
pub struct ProverClient {
    prover: ProverKind,
    #[allow(dead_code)]
    options: EmbeddedOptions,
}

impl Default for ProverClient {
    fn default() -> Self {
        // Since default can't return a Result, we panic if multiple instances are created via this method.
        ensure_single_instance().expect("ProverClient already exists");

        Self {
            prover: ProverClientBuilder::<EmbeddedOptions>::build_emu(None, None)
                .expect("Failed to initialize ProverClient"),
            options: EmbeddedOptions::default(),
        }
    }
}

impl ProverClient {
    /// Start building an embedded client with the given options.
    pub fn embedded(options: EmbeddedOptions) -> EmbeddedClientBuilder {
        ensure_single_instance().expect("ProverClient already exists");

        ProverClientBuilder { executor: Executor::Emulator, gpu_params: None, options }
    }

    // /// Start building a remote client with the given options.
    // pub fn remote(options: RemoteOptions) -> RemoteClientBuilder {
    //     ensure_single_instance()?;

    //     ProverClientBuilder { executor: Executor::Emulator, gpu_params: None, options }
    // }

    pub fn vk(&self, program: &GuestProgram) -> Result<ZiskProgramVK> {
        match &self.prover {
            ProverKind::Emu(p) => p.vk(program),
            ProverKind::Asm(p) => p.vk(program),
        }
    }

    pub fn prove<'a>(&'a self, program: &'a GuestProgram, stdin: ZiskStdin) -> ProveRequest<'a> {
        ProveRequest::new(self, program, stdin)
    }

    pub fn execute<'a>(
        &'a self,
        program: &'a GuestProgram,
        stdin: ZiskStdin,
    ) -> ExecuteRequest<'a> {
        ExecuteRequest::new(self, program, stdin)
    }

    pub fn setup<'a>(&'a self, program: &'a GuestProgram) -> SetupRequest<'a, Self> {
        SetupRequest::new(self, program)
    }

    pub fn upload<'a>(&'a self, program: &'a GuestProgram) -> UploadRequest<'a, Self> {
        UploadRequest::new(self, program)
    }
}

impl ClientConfig for ProverClient {
    fn assembly_enabled(&self) -> bool {
        matches!(self.prover, ProverKind::Asm(_))
    }

    fn default_executor(&self) -> Executor {
        match self.prover {
            ProverKind::Emu(_) => Executor::Emulator,
            ProverKind::Asm(_) => Executor::Assembly,
        }
    }
}

impl ProverClient {
    pub(super) fn run_prove(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        opts: ProofOpts,
    ) -> Result<Proof> {
        let result = match &self.prover {
            ProverKind::Emu(p) => p.prove(program, stdin).with_proof_options(opts).run()?,
            ProverKind::Asm(p) => p.prove(program, stdin).with_proof_options(opts).run()?,
        };
        Ok(Proof::new(result))
    }
}

// ---------------------------------------------------------------------------
// RemoteProverClient
// ---------------------------------------------------------------------------

// /// Remote prover client. Delegates proof generation to a remote coordinator via gRPC.
// pub struct RemoteProverClient {
//     config: Config,
//     #[allow(dead_code)]
//     options: RemoteOptions,
// }

// impl RemoteProverClient {
//     pub fn prove<'a>(
//         &'a self,
//         program: &'a GuestProgram,
//         stdin: ZiskStdin,
//     ) -> ProveRequest<'a, Self> {
//         ProveRequest::new(self, program, stdin)
//     }

//     pub fn execute<'a>(
//         &'a self,
//         program: &'a GuestProgram,
//         stdin: ZiskStdin,
//     ) -> ExecuteRequest<'a, Self> {
//         ExecuteRequest::new(self, program, stdin)
//     }

//     pub fn setup<'a>(&'a self, program: &'a GuestProgram) -> SetupRequest<'a, Self> {
//         SetupRequest::new(self, program)
//     }

//     pub fn upload<'a>(&'a self, program: &'a GuestProgram) -> UploadRequest<'a, Self> {
//         UploadRequest::new(self, program)
//     }
// }

// impl ClientConfig for RemoteProverClient {
//     fn assembly_enabled(&self) -> bool {
//         self.config.executor == Executor::Assembly
//     }

//     fn default_executor(&self) -> Executor {
//         self.config.executor
//     }
// }

// ---------------------------------------------------------------------------
// ProverClientBuilder (EmbeddedClientBuilder + RemoteClientBuilder)
// ---------------------------------------------------------------------------

/// Builder for an embedded `ProverClient`.
pub type EmbeddedClientBuilder = ProverClientBuilder<EmbeddedOptions>;

/// Builder for a `RemoteProverClient`.
// pub type RemoteClientBuilder = ProverClientBuilder<RemoteOptions>;
pub struct ProverClientBuilder<O> {
    executor: Executor,
    gpu_params: Option<ParamsGPU>,
    options: O,
}

impl ProverClientBuilder<EmbeddedOptions> {
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

    /// Declare executor capability.
    ///
    /// `Executor::Assembly` must be declared here to use it at prove time.
    #[must_use]
    pub fn executor(mut self, executor: Executor) -> Self {
        self.executor = executor;
        self
    }

    /// Build an embedded `ProverClient`.
    pub fn build(self) -> Result<ProverClient> {
        let prover = match self.executor {
            Executor::Emulator => {
                Self::build_emu(self.options.proving_key.as_ref(), self.gpu_params)?
            }
            Executor::Assembly => {
                Self::build_asm(self.options.proving_key.as_ref(), self.gpu_params)?
            }
        };
        Ok(ProverClient { prover, options: self.options })
    }

    fn build_emu(
        proving_key: Option<&PathBuf>,
        gpu_params: Option<ParamsGPU>,
    ) -> Result<ProverKind> {
        let pk = get_proving_key(proving_key);
        let pk_snark = get_proving_key_snark(None);
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
        Ok(ProverKind::Emu(PublicZiskProver::new(ZiskProver::<Emu>::new(emu))))
    }

    fn build_asm(
        proving_key: Option<&PathBuf>,
        gpu_params: Option<ParamsGPU>,
    ) -> Result<ProverKind> {
        let pk = get_proving_key(proving_key);
        let pk_snark = get_proving_key_snark(None);
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
        Ok(ProverKind::Asm(PublicZiskProver::new(ZiskProver::<Asm>::new(asm))))
    }
}

// impl ProverClientBuilder<RemoteOptions> {
//     /// Enable GPU acceleration with default parameters.
//     #[must_use]
//     pub fn gpu(mut self) -> Self {
//         self.gpu_params = Some(ParamsGPU::default());
//         self
//     }
//
//     /// Enable GPU acceleration with custom parameters.
//     #[must_use]
//     pub fn with_gpu_params(mut self, gpu_params: ParamsGPU) -> Self {
//         self.gpu_params = Some(gpu_params);
//         self
//     }

//     /// Declare executor capability.
//     ///
//     /// `Executor::Assembly` must be declared here to use it at prove time.
//     #[must_use]
//     pub fn executor(mut self, executor: Executor) -> Self {
//         self.executor = executor;
//         self
//     }

//     /// Build a `RemoteProverClient`.
//     pub fn build(self) -> Result<RemoteProverClient> {
//         Ok(RemoteProverClient {
//             config: Config::new(self.executor, self.use_gpu),
//             options: self.options,
//         })
//     }
// }
