use std::path::PathBuf;

use zisk_prover_backend::{AsmOptions, BackendProverOpts};

/// Public prover configuration for the SDK.
///
/// Controls key paths, memory, parallelism and MPI settings.
/// GPU acceleration is configured separately via [`crate::ProverClientBuilder::gpu`].
/// ASM-specific options are configured via [`crate::ProverClientBuilder::asm_options`].
#[derive(Clone)]
pub struct ProverOpts {
    /// Reduce memory footprint during proving at the cost of speed.
    pub minimal_memory: bool,

    /// Path to the proving key directory.
    pub proving_key: Option<PathBuf>,

    /// Path to the PLONK proving key directory.
    pub proving_key_snark: Option<PathBuf>,

    /// Eagerly preload PLONK/SNARK proving keys at startup.
    pub preload_plonk: bool,

    /// Use shared tables across MPI processes (default: true).
    pub shared_tables: bool,

    /// Use Remote Memory Access for MPI communication.
    pub rma: bool,

    /// Maximum memory (bytes) for witness storage during proving.
    pub max_witness_stored: Option<usize>,

    /// Number of threads for witness generation thread pools.
    pub number_threads_witness: Option<usize>,

    /// Maximum number of parallel streams during proving.
    pub max_streams: Option<usize>,
}

impl Default for ProverOpts {
    fn default() -> Self {
        Self {
            minimal_memory: false,
            proving_key: None,
            proving_key_snark: None,
            preload_plonk: false,
            shared_tables: true,
            rma: false,
            max_witness_stored: None,
            number_threads_witness: None,
            max_streams: None,
        }
    }
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

    /// Configure shared tables for MPI execution.
    #[must_use]
    pub fn shared_tables(mut self, value: bool) -> Self {
        self.shared_tables = value;
        self
    }

    /// Enable Remote Memory Access for MPI communication.
    #[must_use]
    pub fn rma(mut self, value: bool) -> Self {
        self.rma = value;
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
        BackendProverOpts {
            aggregation: true,
            verify_proofs: false,
            minimal_memory: self.minimal_memory,
            output_dir_path: None,
            verbose: 0,
            proving_key: self.proving_key,
            proving_key_snark: self.proving_key_snark,
            plonk: false, // determined by ProofKind, set separately during build
            preload_plonk: self.preload_plonk,
            shared_tables: self.shared_tables,
            rma: self.rma,
            gpu,
            packed: gpu,
            max_witness_stored: self.max_witness_stored,
            number_threads_witness: self.number_threads_witness,
            max_streams: self.max_streams,
            asm_options: AsmOptions::default(),
        }
    }
}
