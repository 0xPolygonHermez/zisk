use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use fields::Goldilocks;
use proofman_common::{MpiCtx, ProofCtx};
use recurser::setup::{run_setup_recurser_aggregator, SetupRecurserAggregatorOptions};
use rom_setup::{rom_merkle_setup, HashMode};
use std::str::FromStr;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::ZiskPaths;
use zisk_prover_backend::{setup_logger, GuestProgram};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate the recurser setup on top of an existing vadcop_final
/// build directory. See recurser/docs/aggregator-flow.md.
pub struct ZiskSetupRecurserAggregator {
    /// ZisK setup directory. Defaults to `~/.zisk`.
    #[arg(short = 's', long = "setup-dir")]
    pub setup_dir: Option<String>,

    /// Where to write the generated artifacts. Must differ from `--setup-dir`.
    #[arg(short = 'o', long = "output-dir", default_value = "build")]
    pub output_dir: String,

    /// Guest program ELFs to register as recurser leaves. Order is significant —
    /// it fixes the `programVKs[]` index of each program.
    #[arg(short = 'e', long = "program-elf", num_args = 1.., required = true)]
    pub program_elfs: Vec<PathBuf>,

    /// `AggregatePublics` Circom body (required).
    #[arg(long = "aggregate-publics-template")]
    pub aggregate_publics_template: String,

    /// `PreparePublics` Circom body. Omit for the built-in identity default.
    #[arg(long = "prepare-publics-template")]
    pub prepare_publics_template: Option<String>,

    /// `CheckPublics` Circom body. Omit for the built-in no-op default.
    #[arg(long = "check-publics-template")]
    pub check_publics_template: Option<String>,

    /// Number of side inputs threaded into the three sub-templates.
    #[arg(long = "n-private-inputs", default_value_t = 0)]
    pub n_private_inputs: usize,

    /// Path to a precomputed proving key
    #[arg(short = 'k', long = "proving-key")]
    pub proving_key: Option<PathBuf>,

    /// Cache directory for rom-setup verkey/bin artifacts. Defaults to `~/.zisk/cache`.
    #[arg(long = "cache-dir")]
    pub cache_dir: Option<PathBuf>,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskSetupRecurserAggregator {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        rayon::ThreadPoolBuilder::new().stack_size(64 * 1024 * 1024).build_global().ok();

        let program_vks = self.derive_program_vks()?;

        let setup_dir = match &self.setup_dir {
            Some(p) => p.clone(),
            None => ZiskPaths::global()
                .home
                .to_str()
                .context("default ~/.zisk path is not valid UTF-8")?
                .to_string(),
        };

        let opts = SetupRecurserAggregatorOptions {
            setup_dir,
            output_dir: self.output_dir.clone(),
            program_vks,
            n_private_inputs: self.n_private_inputs,
            prepare_publics_template: self.prepare_publics_template.clone(),
            check_publics_template: self.check_publics_template.clone(),
            aggregate_publics_template: self.aggregate_publics_template.clone(),
        };

        run_setup_recurser_aggregator(&opts)
    }

    fn derive_program_vks(&self) -> Result<Vec<[String; 4]>> {
        let proving_key = ZiskPaths::get_proving_key(self.proving_key.as_ref());
        let mpi_ctx = Arc::new(MpiCtx::new());
        let pctx = ProofCtx::<Goldilocks>::create_ctx(
            proving_key,
            false,
            self.verbose.into(),
            mpi_ctx,
            false,
        )?;

        // Program VKs must be derived under the same hash family the recurser
        // (and its proving key) uses, so they match at membership-check time.
        let hash_mode = HashMode::from_str(&pctx.global_info.hash).map_err(|e| {
            anyhow::anyhow!(
                "proving key global_info.hash {:?} is not a recognized HashMode: {e}",
                pctx.global_info.hash
            )
        })?;

        let mut vks: Vec<[String; 4]> = Vec::with_capacity(self.program_elfs.len());
        for elf_path in &self.program_elfs {
            tracing::info!("Deriving program VK from ELF: {}", elf_path.display());
            let guest_program = GuestProgram::from_uri(
                elf_path
                    .to_str()
                    .with_context(|| format!("Non-UTF-8 ELF path: {}", elf_path.display()))?,
            )?;
            let program_vk = rom_merkle_setup::<Goldilocks>(
                &pctx,
                guest_program.elf(),
                &self.cache_dir,
                false,
                hash_mode,
            )
            .with_context(|| format!("rom_merkle_setup failed for {}", elf_path.display()))?;
            let limbs: [String; 4] = <[u64; 4]>::try_from(program_vk.vk.as_slice())
                .with_context(|| {
                    format!("VK from {} did not decode into 4 u64 limbs", elf_path.display())
                })?
                .map(|w| w.to_string());
            // Duplicates would break the membership check's soundness
            // (product-of-(1-eq) assumes uniqueness).
            if let Some(prior_idx) = vks.iter().position(|existing| existing == &limbs) {
                bail!(
                    "Duplicate program VK at --program-elf #{} ({}); already registered at #{} ({})",
                    vks.len(),
                    elf_path.display(),
                    prior_idx,
                    self.program_elfs[prior_idx].display(),
                );
            }
            vks.push(limbs);
        }
        Ok(vks)
    }
}
