use std::path::PathBuf;

use zisk_prover_backend::BackendProverOpts;

/// Public prover configuration for the SDK.
///
/// Controls key paths, memory, parallelism and MPI settings.
/// GPU acceleration is configured separately via [`crate::EmbeddedClientBuilder::gpu`].
/// ASM-specific options are configured via [`crate::EmbeddedClientBuilder::asm_options`].
#[derive(Default, Clone)]
pub struct EmbeddedOpts {
    /// Reduce memory footprint during proving at the cost of speed.
    pub minimal_memory: bool,

    /// Path to the proving key directory.
    pub proving_key: Option<PathBuf>,

    /// Path to the SNARK proving key directory, set up for one final SNARK protocol.
    pub proving_key_snark: Option<PathBuf>,

    /// Eagerly preload the SNARK proving key at startup.
    pub preload_snark: bool,

    /// Maximum memory (bytes) for witness storage during proving.
    pub max_witness_stored: Option<usize>,

    /// Number of threads for witness generation thread pools.
    pub number_threads_witness: Option<usize>,

    /// Maximum number of parallel streams during proving.
    pub max_streams: Option<usize>,

    /// Maximum number of per-GPU recursive (aggregation) streams during proving.
    pub max_recursive_streams: Option<usize>,
}

impl EmbeddedOpts {
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

    /// Set the path to the SNARK proving key directory.
    #[must_use]
    pub fn proving_key_snark(mut self, path: impl Into<PathBuf>) -> Self {
        self.proving_key_snark = Some(path.into());
        self
    }

    /// Eagerly preload the SNARK proving key at startup.
    #[must_use]
    pub fn preload_snark(mut self) -> Self {
        self.preload_snark = true;
        self
    }

    /// Eagerly preload the SNARK proving key at startup.
    #[deprecated(note = "the key may be of any final SNARK: use `preload_snark`")]
    #[must_use]
    pub fn preload_plonk(self) -> Self {
        self.preload_snark()
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

    /// Set the maximum number of per-GPU recursive (aggregation) streams during proving.
    #[must_use]
    pub fn max_recursive_streams(mut self, max: usize) -> Self {
        self.max_recursive_streams = Some(max);
        self
    }

    /// The backend options of these, for a prover that runs the final SNARK when `final_snark`.
    pub(crate) fn into_backend_opts(self, gpu: bool, final_snark: bool) -> BackendProverOpts {
        let mut opts = BackendProverOpts::default().aggregation(true);

        if self.minimal_memory {
            opts = opts.minimal_memory();
        }

        if let Some(pk) = self.proving_key {
            opts = opts.proving_key(pk);
        }

        if let Some(pk_snark) = self.proving_key_snark {
            opts = opts.proving_key_snark(pk_snark);
        }

        // A final SNARK (pilfflonk, or PLONK: the protocol of the SNARK proving key) is declared to
        // proofman, as the worker and `cargo-zisk-dev prove` do: proofman sets the GPU arena up for
        // the wrap, and a blake3 key needs it to prove the recursivef its SNARK wraps.
        if final_snark || self.preload_snark {
            opts = opts.snark(self.preload_snark);
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

        if let Some(max) = self.max_recursive_streams {
            opts = opts.max_recursive_streams(max);
        }

        opts
    }
}

#[cfg(test)]
mod tests {
    use super::EmbeddedOpts;

    #[test]
    fn a_final_snark_is_declared_to_proofman() {
        let opts = EmbeddedOpts::default().into_backend_opts(false, true);
        assert!(opts.build_proofman_options().final_snark.is_some());
        assert!(!opts.preload_snark());
    }

    #[test]
    fn a_preloaded_snark_key_is_a_final_snark() {
        let opts = EmbeddedOpts::default().preload_snark().into_backend_opts(false, true);
        assert!(opts.build_proofman_options().final_snark.is_some());
        assert!(opts.preload_snark());
    }

    #[test]
    fn a_stark_prover_runs_no_final_snark() {
        let opts = EmbeddedOpts::default().into_backend_opts(false, false);
        assert!(opts.build_proofman_options().final_snark.is_none());
    }
}
