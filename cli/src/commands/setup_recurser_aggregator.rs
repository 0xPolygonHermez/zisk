use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use fields::Goldilocks;
use proofman_common::{MpiCtx, ProofCtx};
use recurser::setup::{run_setup_recurser_aggregator, SetupRecurserAggregatorOptions};
use recurser::{CircomTemplates, NormalizeGroup};
use rom_setup::{rom_merkle_setup, HashMode};
use std::str::FromStr;
use zisk_build::{guest_elf_map, resolve_aggregation, ResolvedAggregation, ZISK_VERSION_MESSAGE};
use zisk_common::ZiskPaths;
use zisk_prover_backend::{setup_logger, GuestProgram};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Generate the recurser setup for an aggregation program on top of an
/// existing vadcop_final build directory. See recurser/docs/aggregator-flow.md.
///
/// The input is the same `programs/aggregations/<name>.toml` the build
/// pipeline consumes for `load_aggregation_program!`; the referenced guest
/// programs must already be built (`cargo build` of the host crate).
pub struct ZiskSetupRecurserAggregator {
    /// Aggregation definition: `<programs>/aggregations/<name>.toml`.
    #[arg(short = 'a', long = "aggregation")]
    pub aggregation: PathBuf,

    /// Resolve guest ELFs from the release profile instead of debug.
    #[arg(long, default_value_t = false)]
    pub release: bool,

    /// ZisK setup directory. Defaults to `~/.zisk`.
    #[arg(short = 's', long = "setup-dir")]
    pub setup_dir: Option<String>,

    /// Where to write the generated artifacts. Must differ from `--setup-dir`.
    #[arg(short = 'o', long = "output-dir", default_value = "build")]
    pub output_dir: String,

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

        // The definition lives at `<programs>/aggregations/<name>.toml`, so
        // the guest workspace is two levels up.
        let programs_dir = self
            .aggregation
            .canonicalize()
            .with_context(|| format!("definition not found: {}", self.aggregation.display()))?
            .parent()
            .and_then(|aggregations| aggregations.parent())
            .context("definition must live under <programs>/aggregations/")?
            .to_path_buf();

        let elf_map = guest_elf_map(&programs_dir, self.release)?;
        let (definition, _circuit_paths) = resolve_aggregation(&self.aggregation, &elf_map)
            .with_context(|| format!("aggregation definition {}", self.aggregation.display()))?;
        tracing::info!("Aggregation program: {}", definition.name);

        let program_vks = self.derive_program_vks(&definition)?;

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
            templates: CircomTemplates {
                normalize_groups: definition
                    .normalize_groups
                    .into_iter()
                    .map(|g| NormalizeGroup {
                        member_indices: g.member_indices,
                        body: g.body,
                        n_free_inputs: g.n_free_inputs,
                    })
                    .collect(),
                aggregate_publics: definition.aggregate_publics_body,
            },
        };

        run_setup_recurser_aggregator(&opts)
    }

    fn derive_program_vks(&self, definition: &ResolvedAggregation) -> Result<Vec<[String; 4]>> {
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

        let mut vks: Vec<[String; 4]> = Vec::with_capacity(definition.programs.len());
        for program in &definition.programs {
            tracing::info!("Deriving VK for program '{}' ({})", program.name, program.elf_path);
            let guest_program = GuestProgram::from_uri(&program.elf_path)?;
            let program_vk = rom_merkle_setup::<Goldilocks>(
                &pctx,
                guest_program.elf(),
                &self.cache_dir,
                false,
                hash_mode,
            )
            .with_context(|| format!("rom_merkle_setup failed for '{}'", program.name))?;
            let limbs: [String; 4] = <[u64; 4]>::try_from(program_vk.vk.as_slice())
                .with_context(|| {
                    format!("VK for '{}' did not decode into 4 u64 limbs", program.name)
                })?
                .map(|w| w.to_string());
            // Duplicates would break the membership check's soundness
            // (product-of-(1-eq) assumes uniqueness).
            if let Some(prior_idx) = vks.iter().position(|existing| existing == &limbs) {
                bail!(
                    "Duplicate program VK: '{}' and '{}' resolve to the same VK",
                    definition.programs[prior_idx].name,
                    program.name,
                );
            }
            vks.push(limbs);
        }
        Ok(vks)
    }
}
