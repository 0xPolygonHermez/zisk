use anyhow::Result;
use colored::Colorize;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::{PlonkVkey, Proof};
use zisk_prover_backend::setup_logger;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Verify a proof
pub(crate) struct VerifyCmd {
    /// Path to the proof file
    #[clap(short = 'p', long)]
    proof: String,

    /// Trusted PLONK circuit key (`final.verkey.json`). If omitted, this release's key for
    /// the proof's family; the key the proof carries is never used.
    #[clap(short = 'k', long = "plonk-vk")]
    plonk_vk: Option<String>,

    /// Trusted pilfflonk key (`pilfflonk.vkey.json`). No release publishes one yet, so a
    /// pilfflonk proof needs it.
    #[clap(long = "pilfflonk-vk")]
    pilfflonk_vk: Option<String>,

    /// Trusted recursion key (a `verkey.json`: JSON array of 4 u64 limbs) of the recurser
    /// that produced an aggregated proof. If omitted, a plain proof verifies under this
    /// release's key for its family and stage.
    #[clap(long = "setup-vk", conflicts_with = "leaf_setup_vk")]
    setup_vk: Option<String>,

    /// Leaf key of a non-release setup: its `vadcop_final.verkey.json`, or
    /// `vadcop_final_compressed.verkey.json` for a compressed proof.
    #[clap(long = "leaf-setup-vk")]
    leaf_setup_vk: Option<String>,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl VerifyCmd {
    pub(crate) fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        tracing::info!(
            "{}",
            format!("{} ZiskVerify", format!("{: >12}", "Command").bright_green().bold())
        );
        tracing::info!("");

        let start = std::time::Instant::now();

        let proof = Proof::load(&self.proof)
            .map_err(|e| anyhow::anyhow!("Error loading proof from {}: {}", self.proof, e))?;

        let proof_type = crate::proof::verify_kind_label(&proof);

        let trusted_plonk_vk =
            match &self.plonk_vk {
                Some(path) => Some(PlonkVkey::load(path).map_err(|e| {
                    anyhow::anyhow!("Error loading PLONK vkey from {}: {}", path, e)
                })?),
                None => None,
            };

        let trusted_pilfflonk_vk: Option<String> = match &self.pilfflonk_vk {
            Some(path) => Some(std::fs::read_to_string(path).map_err(|e| {
                anyhow::anyhow!("Error reading pilfflonk vkey from {}: {}", path, e)
            })?),
            None => None,
        };

        let trusted_setup_vk: Option<Vec<u64>> =
            match self.setup_vk.as_ref().or(self.leaf_setup_vk.as_ref()) {
                Some(path) => {
                    let contents = std::fs::read_to_string(path).map_err(|e| {
                        anyhow::anyhow!("Error reading setup vkey from {}: {}", path, e)
                    })?;
                    Some(serde_json::from_str(&contents).map_err(|e| {
                        anyhow::anyhow!(
                            "Error parsing setup vkey from {} (expected a JSON array of u64): {}",
                            path,
                            e
                        )
                    })?)
                }
                None => None,
            };

        let mut builder = proof.verify_builder();
        if let Some(vkey) = &trusted_plonk_vk {
            builder = builder.with_plonk_vk(vkey);
        }
        if let Some(vkey) = &trusted_pilfflonk_vk {
            builder = builder.with_pilfflonk_vkey(vkey);
        }
        if let Some(vk) = &trusted_setup_vk {
            builder = if self.leaf_setup_vk.is_some() {
                builder.with_leaf_setup_vk(vk)
            } else {
                builder.with_setup_vk(vk)
            };
        }
        let result = builder.verify();

        let elapsed = start.elapsed();

        if result.is_err() {
            tracing::info!(
                "{}",
                format!("\u{2717} {} proof was not verified", proof_type).bright_red().bold()
            );
        } else {
            tracing::info!(
                "{}",
                format!("\u{2713} {} proof was verified", proof_type).bright_green().bold()
            );
        }

        tracing::info!("{}", "--- VERIFICATION SUMMARY ---".bright_green().bold());
        tracing::info!("      time: {} milliseconds", elapsed.as_millis());
        tracing::info!("{}", "----------------------------".bright_green().bold());

        result.map_err(Into::into)
    }
}
