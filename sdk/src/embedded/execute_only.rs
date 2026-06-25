//! Execute-only embedded client.
//!
//! [`EmbeddedExecuteOnlyClient`] runs guest programs **without loading
//! proving keys**, mirroring the CLI's `--standalone` mode. Intended for
//! cargo tests and dev iteration that need to emulate + plan but not
//! prove.
//!
//! Built via [`crate::ProverClient::embedded`]`().execute_only().build()`.

use std::path::PathBuf;
use std::sync::Arc;

use zisk_prover_backend::{
    AsmOptions, BackendProverOpts, ExecuteClient, ExecuteOutput, GuestProgram, ProverClientBuilder,
};

use crate::{ExecutorKind, Result, SdkError, ZiskHints, ZiskStdin};

/// Builder for an [`EmbeddedExecuteOnlyClient`].
///
/// Reached via `ProverClient::embedded().execute_only()` or
/// `EmbeddedClientBuilder::execute_only()`.
pub struct EmbeddedExecuteOnlyBuilder {
    executor: ExecutorKind,
    asm_options: Option<AsmOptions>,
    verbose: u8,
}

impl Default for EmbeddedExecuteOnlyBuilder {
    fn default() -> Self {
        Self { executor: ExecutorKind::Emulator, asm_options: None, verbose: 0 }
    }
}

impl EmbeddedExecuteOnlyBuilder {
    /// Initial state — emulator backend, default options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cross-constructor used by `EmbeddedClientBuilder::execute_only()`.
    pub(crate) fn from_parts(executor: ExecutorKind, asm_options: Option<AsmOptions>) -> Self {
        Self { executor, asm_options, verbose: 0 }
    }

    /// Use the Emulator backend (default).
    #[must_use]
    pub fn emulator(mut self) -> Self {
        self.executor = ExecutorKind::Emulator;
        self
    }

    /// Use the Assembly backend.
    #[must_use]
    pub fn assembly(mut self) -> Self {
        self.executor = ExecutorKind::Assembly;
        self
    }

    /// Override ASM-specific options. Only meaningful with `.assembly()`.
    #[must_use]
    pub fn asm_options(mut self, opts: AsmOptions) -> Self {
        self.asm_options = Some(opts);
        self
    }

    /// Set the cache directory for generated ASM binaries. Only
    /// meaningful with `.assembly()`. Equivalent to setting
    /// `AsmOptions::asm_path`.
    #[must_use]
    pub fn asm_cache_dir(mut self, path: impl Into<PathBuf>) -> Self {
        let mut opts = self.asm_options.take().unwrap_or_default();
        opts = opts.asm_path(path.into());
        self.asm_options = Some(opts);
        self
    }

    /// Verbosity level (0–2 maps to Info / Debug / Trace).
    #[must_use]
    pub fn verbose(mut self, v: u8) -> Self {
        self.verbose = v;
        self
    }

    /// Build the client.
    pub fn build(self) -> Result<EmbeddedExecuteOnlyClient> {
        crate::client::ensure_single_instance();

        let mut backend_opts = BackendProverOpts::default().verbose(self.verbose);
        if let Some(asm_opts) = self.asm_options {
            backend_opts = backend_opts.with_asm_options(asm_opts);
        }

        let prover: Box<dyn ExecuteClient + Send + Sync> = match self.executor {
            ExecutorKind::Emulator => Box::new(
                ProverClientBuilder::new()
                    .emu()
                    .with_prover_options(backend_opts)
                    .execute_only()
                    .build()
                    .map_err(SdkError::backend)?,
            ),
            ExecutorKind::Assembly => Box::new(
                ProverClientBuilder::new()
                    .asm()
                    .with_prover_options(backend_opts)
                    .execute_only()
                    .build()
                    .map_err(SdkError::backend)?,
            ),
        };

        Ok(EmbeddedExecuteOnlyClient { prover: Arc::from(prover), executor: self.executor })
    }
}

/// Synchronous execute-only client. No proving keys loaded.
///
/// Designed for cargo tests and dev iteration. Setup the program once,
/// execute many times with different stdins.
///
/// # Example
/// ```ignore
/// use zisk_sdk::{ProverClient, GuestProgram, ZiskStdin};
///
/// let client = ProverClient::embedded().assembly().execute_only().build()?;
/// let program = GuestProgram::from_uri("path/to/elf")?;
///
/// client.setup(&program, false)?;
/// let out = client.execute(&program, ZiskStdin::new(), None)?;
/// assert!(out.get_execution_steps() > 0);
/// ```
pub struct EmbeddedExecuteOnlyClient {
    prover: Arc<dyn ExecuteClient + Send + Sync>,
    executor: ExecutorKind,
}

impl Clone for EmbeddedExecuteOnlyClient {
    fn clone(&self) -> Self {
        Self { prover: Arc::clone(&self.prover), executor: self.executor }
    }
}

impl EmbeddedExecuteOnlyClient {
    /// Backend selected on the builder.
    pub fn executor(&self) -> ExecutorKind {
        self.executor
    }

    /// Prepare the client to execute `program`. Idempotent per program
    /// (caches by program id internally). `with_hints` is meaningful
    /// only with the Assembly backend; the Emulator ignores it.
    pub fn setup(&self, program: &GuestProgram, with_hints: bool) -> Result<()> {
        self.prover.setup(program, with_hints).map_err(SdkError::backend)
    }

    /// Run a single execution. `program` must match the one passed to
    /// `setup()`. Errors if `hints` is set with the Emulator backend —
    /// mirroring the full embedded path's behavior.
    pub fn execute(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        hints: Option<ZiskHints>,
    ) -> Result<ExecuteOutput> {
        if hints.is_some() && self.executor == ExecutorKind::Emulator {
            return Err(SdkError::UnsupportedExecutor(
                "Hints require the Assembly executor".to_string(),
            ));
        }
        self.prover
            .execute(program, stdin.into_inner(), hints.map(ZiskHints::into_inner))
            .map_err(SdkError::backend)
    }
}
