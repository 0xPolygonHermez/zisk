use std::path::PathBuf;

use zisk_prover_backend::BackendProverOpts;

/// Public prover configuration for the SDK.
///
/// Controls key paths, memory, parallelism and MPI settings.
/// GPU acceleration is configured separately via [`crate::ProverClientBuilder::gpu`].
/// ASM-specific options are configured via [`crate::ProverClientBuilder::asm_options`].
#[derive(Default, Clone)]
pub struct ProverOpts {
    /// Reduce memory footprint during proving at the cost of speed.
    pub minimal_memory: bool,

    /// Path to the proving key directory.
    pub proving_key: Option<PathBuf>,

    /// Path to the PLONK proving key directory.
    pub proving_key_snark: Option<PathBuf>,

    /// Eagerly preload PLONK/SNARK proving keys at startup.
    pub preload_plonk: bool,

    /// Maximum memory (bytes) for witness storage during proving.
    pub max_witness_stored: Option<usize>,

    /// Number of threads for witness generation thread pools.
    pub number_threads_witness: Option<usize>,

    /// Maximum number of parallel streams during proving.
    pub max_streams: Option<usize>,
}

impl ProverOpts {
    /// Reduce memory footprint during proving at the cost of speed.
    #[must_use]
    pub fn minimal_memory(mut self) -> Self {
        self.minimal_memory = true;
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
    pub fn proving_key_snark(mut self, path: impl Into<PathBuf>) -> Self {
        self.proving_key_snark = Some(path.into());
        self
    }

    /// Eagerly preload PLONK/SNARK proving keys at startup.
    #[must_use]
    pub fn preload_plonk(mut self) -> Self {
        self.preload_plonk = true;
        self
    }

    /// Set the maximum memory (bytes) for witness storage during proving.
    #[must_use]
    pub fn max_witness_stored(mut self, max: usize) -> Self {
        self.max_witness_stored = Some(max);
        self
    }

    /// Set the number of threads for witness generation thread pools.
    #[must_use]
    pub fn number_threads_witness(mut self, threads: usize) -> Self {
        self.number_threads_witness = Some(threads);
        self
    }

    /// Set the maximum number of parallel streams during proving.
    #[must_use]
    pub fn max_streams(mut self, max: usize) -> Self {
        self.max_streams = Some(max);
        self
    }

    pub(crate) fn into_backend_opts(self, gpu: bool) -> BackendProverOpts {
        let mut opts = BackendProverOpts::default().aggregation(true);

        if self.minimal_memory {
            opts = opts.minimal_memory();
        }

        if let Some(pk) = self.proving_key {
            opts = opts.proving_key(pk);
        }

        if let Some(pk_snark) = self.proving_key_snark {
            opts = opts.proving_key_plonk(pk_snark);
        }

        if self.preload_plonk {
            opts = opts.plonk(true);
        }

        if gpu {
            opts = opts.gpu().packed();
        }

        if let Some(max) = self.max_witness_stored {
            opts = opts.max_witness_stored(max);
        }

        if let Some(threads) = self.number_threads_witness {
            opts = opts.number_threads_witness(threads);
        }

        if let Some(max) = self.max_streams {
            opts = opts.max_streams(max);
        }

        opts
    }
}
