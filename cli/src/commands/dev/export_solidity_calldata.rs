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
/// Export the four ABI fields of a final SNARK proof (PLONK or pilfflonk) as JSON for the Solidity
/// verifier.
///
/// Test fixture only: `programVK` and `rootCVadcopFinal` are read out of the proof
/// itself. A production integration must hold both as contract constants, or
/// verification is self-keyed.
pub(crate) struct ExportSolidityCalldataCmd {
    /// Path to the final SNARK proof file (`prove`/`wrap` with `--pilfflonk` or `--plonk`)
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
                (proof_bytes.clone(), rootc.as_slice(), publics_full.as_slice())
            }
            ProofBody::Pilfflonk { proof_bytes, vkey_json, publics_full, rootc, .. } => (
                pilfflonk_calldata(
                    proof_bytes,
                    vkey_json,
                    publics_full,
                    rootc,
                    zisk_verifier::pilfflonk_vkey_digest(proof.get_program_vk().hash_mode.as_str()),
                )?,
                rootc.as_slice(),
                publics_full.as_slice(),
            ),
            _ => {
                return Err(anyhow!(
                    "Expected a final SNARK proof; got {:?}. Prove or wrap with --pilfflonk or --plonk first.",
                    proof.kind()
                ));
            }
        };
        let program_vk = proof.get_program_vk();
        if program_vk.vk.len() != 4 {
            return Err(anyhow!(
                "program_vk has unexpected length {} (expected 4 u64s)",
                program_vk.vk.len()
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
        // `snark_inputs_bytes` and `snark_publics_hash` share proofman's circuit layout.
        // The committed identity, not the untrusted outer copy: a relabelled proof
        // would otherwise export calldata the proof does not commit to.
        let program_vk_bytes = u64_chunks_to_be(&program_vk.vk);
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

/// `verifyProof`'s `bytes32[W]` words of a pilfflonk proof. A proof wrapped under another key
/// than this release's (`release_digest`) is refused here, not as a bare `InvalidProof` on-chain.
fn pilfflonk_calldata(
    proof_bytes: &[u8],
    vkey_json: &str,
    publics_full: &[u64],
    rootc: &[u64],
    release_digest: Option<&str>,
) -> Result<Vec<u8>> {
    use proofman_pilfflonk::{Calldata, FrBytes, Proof as PilfflonkProof, ProofNames, Vkey};

    // Before snark_publics_hash, which expects four.
    if rootc.len() != 4 {
        return Err(anyhow!("rootc has unexpected length {} (expected 4 u64s)", rootc.len()));
    }
    let vkey: Vkey =
        serde_json::from_str(vkey_json).context("the proof's pilfflonk vkey does not parse")?;
    vkey.check_digest().map_err(|e| anyhow!("the proof's pilfflonk vkey: {e}"))?;
    if let Some(release) = release_digest {
        let digest = vkey.digest.to_hex();
        if digest != release {
            return Err(anyhow!(
                "the proof is wrapped under pilfflonk key {digest}, not this release's {release}: \
                 the release's verifier contract would refuse it"
            ));
        }
    }
    let names =
        ProofNames::of_vkey(&vkey).map_err(|e| anyhow!("the proof's pilfflonk vkey: {e}"))?;
    let proof = PilfflonkProof::from_bytes(proof_bytes, &names.shape())
        .map_err(|e| anyhow!("the pilfflonk proof bytes: {e}"))?;
    let digest: [u8; 32] = zisk_common::snark_publics_hash(publics_full, rootc)
        .try_into()
        .map_err(|_| anyhow!("the public digest is not 32 bytes"))?;
    let public = FrBytes::from_be_bytes(digest).map_err(|e| anyhow!("the public digest: {e}"))?;
    let calldata = Calldata::encode(&vkey, &proof, &[public])
        .map_err(|e| anyhow!("pilfflonk calldata: {e}"))?;
    Ok(calldata.proof.concat())
}

fn u64_chunks_to_be(values: &[u64]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, val) in values.iter().enumerate() {
        out[i * 8..(i + 1) * 8].copy_from_slice(&val.to_be_bytes());
    }
    out
}
