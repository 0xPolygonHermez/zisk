use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::{Proof, ProofBody};
use zisk_prover_backend::setup_logger;

use crate::ux::{print_banner, print_banner_command};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Export the four ABI fields of a wrapped PLONK proof as JSON for the Solidity verifier.
pub(crate) struct ExportSolidityCalldataCmd {
    /// Path to the wrapped PLONK proof file (output of `cargo-zisk wrap-proof --plonk`)
    #[arg(short = 'p', long)]
    proof: PathBuf,

    /// Output path for the JSON fixture
    #[arg(short = 'o', long)]
    output: PathBuf,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Serialize)]
struct SolidityFixture {
    #[serde(rename = "programVK")]
    program_vk: String,
    #[serde(rename = "rootCVadcopFinal")]
    root_c_vadcop_final: String,
    #[serde(rename = "publicValues")]
    public_values: String,
    #[serde(rename = "proofBytes")]
    proof_bytes: String,
}

impl ExportSolidityCalldataCmd {
    pub(crate) fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        print_banner();
        print_banner_command("Export Solidity Calldata");

        let proof = Proof::load(&self.proof).map_err(|e| {
            anyhow!("Failed to load Proof from file {}: {}", self.proof.display(), e)
        })?;

        let (proof_bytes, rootc, publics_full) = match &proof.body {
            ProofBody::Plonk { proof_bytes, publics_full, rootc, .. } => {
                (proof_bytes.as_slice(), rootc.as_slice(), publics_full.as_slice())
            }
            _ => {
                return Err(anyhow!(
                    "Expected a Plonk-wrapped proof; got {:?}. Run `cargo-zisk wrap-proof --plonk` first.",
                    proof.kind()
                ));
            }
        };
        if proof.program_vk.vk.len() != 4 {
            return Err(anyhow!(
                "program_vk has unexpected length {} (expected 4 u64s)",
                proof.program_vk.vk.len()
            ));
        }

        if rootc.len() != 4 {
            return Err(anyhow!("rootc has unexpected length {} (expected 4 u64s)", rootc.len()));
        }

        // The on-chain verifier hashes sha256(programVK ‖ publicValues ‖ rootCVadcopFinal),
        // which must reproduce the circuit's getSha256Inputs preimage:
        //   - programVK  = rom_root  = program VK, 4×u64 big-endian (32 bytes)
        //   - publicValues = inputs  = the ZISK_PUBLICS user publics, each u64 byte-swizzled
        //                              (ZISK_PUBLICS*8 bytes) — NOT the u32 `PublicValues` view
        //   - rootCVadcopFinal       = the STAMPED verkey (`rootc`): vadcop_final for a plain
        //                              proof, the recurser's own verkey for an aggregated one;
        //                              4×u64 big-endian (32 bytes)
        // The publicValues encoding comes straight from `snark_inputs_bytes` so it stays in
        // lockstep with `snark_publics_hash` (the off-chain/snarkjs path).
        let program_vk_bytes = u64_chunks_to_be(&proof.program_vk.vk);
        let publics_bytes = zisk_common::snark_inputs_bytes(publics_full);
        let root_c_bytes = u64_chunks_to_be(rootc);

        let fixture = SolidityFixture {
            program_vk: format!("0x{}", hex::encode(program_vk_bytes)),
            root_c_vadcop_final: format!("0x{}", hex::encode(root_c_bytes)),
            public_values: format!("0x{}", hex::encode(publics_bytes)),
            proof_bytes: format!("0x{}", hex::encode(proof_bytes)),
        };

        if let Some(parent) = self.output.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create parent directory for {}", self.output.display())
            })?;
        }
        let file = std::fs::File::create(&self.output)
            .with_context(|| format!("failed to create fixture file {}", self.output.display()))?;
        serde_json::to_writer_pretty(file, &fixture).with_context(|| {
            format!("failed to write fixture JSON to {}", self.output.display())
        })?;

        info!("Solidity fixture written to {}", self.output.display());
        Ok(())
    }
}

fn u64_chunks_to_be(values: &[u64]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, val) in values.iter().enumerate() {
        out[i * 8..(i + 1) * 8].copy_from_slice(&val.to_be_bytes());
    }
    out
}
