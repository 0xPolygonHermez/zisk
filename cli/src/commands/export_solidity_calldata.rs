use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::{Proof, ProofKind, ZISK_PUBLICS};
use zisk_prover_backend::setup_logger;

use crate::ux::{print_banner, print_banner_command};

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Export the four ABI fields of a wrapped PLONK proof as JSON for the Solidity verifier.
pub struct ZiskExportSolidityCalldata {
    /// Path to the wrapped PLONK proof file (output of `cargo-zisk wrap-proof --plonk`)
    #[arg(short = 'p', long)]
    pub proof: PathBuf,

    /// Output path for the JSON fixture
    #[arg(short = 'o', long)]
    pub output: PathBuf,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
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

impl ZiskExportSolidityCalldata {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        print_banner();
        print_banner_command("Export Solidity Calldata");

        let proof = Proof::load(&self.proof).map_err(|e| {
            anyhow!("Failed to load Proof from file {}: {}", self.proof.display(), e)
        })?;

        if proof.proof_kind != ProofKind::Plonk {
            return Err(anyhow!(
                "Expected a Plonk-wrapped proof; got {:?}. Run `cargo-zisk wrap-proof --plonk` first.",
                proof.proof_kind
            ));
        }

        if proof.program_vk.vk.len() != 32 {
            return Err(anyhow!(
                "program_vk has unexpected length {} (expected 32 bytes)",
                proof.program_vk.vk.len()
            ));
        }

        // Parse the zisk_vk blob: [vk_len: u32 LE][vadcop_vk_bytes][plonk_vkey_json]
        if proof.zisk_vk.len() < 4 {
            return Err(anyhow!("zisk_vk too short for Plonk proof"));
        }
        let vk_len = u32::from_le_bytes(proof.zisk_vk[0..4].try_into().unwrap()) as usize;
        if proof.zisk_vk.len() < 4 + vk_len {
            return Err(anyhow!("zisk_vk truncated: declared len {} > remaining bytes", vk_len));
        }
        let vadcop_vk = &proof.zisk_vk[4..4 + vk_len];
        if vadcop_vk.len() != 32 {
            return Err(anyhow!(
                "vadcop_vk has unexpected length {} (expected 32 bytes)",
                vadcop_vk.len()
            ));
        }

        // Canonical Solidity layout: [programVK (32) || publicValues (ZISK_PUBLICS*4) || rootCVadcopFinal (32)].
        // This is the exact byte string the on-chain verifier hashes — anchor everything off it.
        let canonical = proof.publics.bytes_solidity(&proof.program_vk, vadcop_vk);
        let publics_data_len = ZISK_PUBLICS * 4;
        let expected_len = 32 + publics_data_len + 32;
        if canonical.len() != expected_len {
            return Err(anyhow!(
                "bytes_solidity returned {} bytes, expected {}",
                canonical.len(),
                expected_len
            ));
        }

        let program_vk_bytes: [u8; 32] = canonical[..32].try_into().unwrap();
        let publics_bytes = &canonical[32..32 + publics_data_len];
        let root_c_bytes: [u8; 32] = canonical[32 + publics_data_len..].try_into().unwrap();

        // Sanity check: the prefix/suffix should be the LE→BE u64 transform of program_vk and
        // vadcop_vk respectively. Recompute them independently and bail on any drift —
        // catches a divergence in `bytes_solidity` before the Hardhat test reverts.
        let independent_prefix = u64_chunks_le_to_be(&proof.program_vk.vk);
        let independent_suffix = u64_chunks_le_to_be(vadcop_vk);
        if independent_prefix != program_vk_bytes || independent_suffix != root_c_bytes {
            return Err(anyhow!(
                "internal: bytes_solidity prefix/suffix diverge from independent LE->BE encoding"
            ));
        }

        let fixture = SolidityFixture {
            program_vk: format!("0x{}", hex::encode(program_vk_bytes)),
            root_c_vadcop_final: format!("0x{}", hex::encode(root_c_bytes)),
            public_values: format!("0x{}", hex::encode(publics_bytes)),
            proof_bytes: format!("0x{}", hex::encode(&proof.proof_bytes)),
        };

        if let Some(parent) = self.output.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create parent directory for {}", self.output.display())
            })?;
        }
        let file = std::fs::File::create(&self.output).with_context(|| {
            format!("failed to create fixture file {}", self.output.display())
        })?;
        serde_json::to_writer_pretty(file, &fixture).with_context(|| {
            format!("failed to write fixture JSON to {}", self.output.display())
        })?;

        info!("Solidity fixture written to {}", self.output.display());
        Ok(())
    }
}

fn u64_chunks_le_to_be(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, chunk) in bytes.chunks_exact(8).enumerate() {
        let val = u64::from_le_bytes(chunk.try_into().unwrap());
        out[i * 8..(i + 1) * 8].copy_from_slice(&val.to_be_bytes());
    }
    out
}
