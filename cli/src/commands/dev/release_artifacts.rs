//! `cargo-zisk-dev release-artifacts`: install a family's setup as the release verification
//! artifacts (driven by `tools/release-artifacts/regen.sh`). `render` rewrites keys.rs from the
//! given manifests, keeping the built-in keys of the other families.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::{Proof, ProofBody, ProofKind};

/// Hash tag, file stem, keys.rs suffix.
const FAMILIES: [(&str, &str, &str); 3] = [
    ("Poseidon1", "poseidon1", "POSEIDON1"),
    ("Poseidon2", "poseidon2", "POSEIDON2"),
    ("blake3", "blake3", "BLAKE3"),
];

const GENERATED: &str =
    "// Written by `cargo-zisk-dev release-artifacts render` (tools/release-artifacts/regen.sh).";

const STAGES: [(&str, &str); 2] = [("vadcop_final", "FRI"), ("vadcop_final_compressed", "STIR")];

const LOW_DEGREE_TEST: &str = STAGES[1].1;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReleaseManifest {
    pub(crate) hash: String,
    pub(crate) vadcop_final_vk: [u64; 4],
    pub(crate) vadcop_final_compressed_vk: [u64; 4],
    pub(crate) pilfflonk_vkey_digest: Option<String>,
    /// The compressed stage's low-degree test.
    pub(crate) low_degree_test: String,
}

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Install a family's release verification artifacts from its setup
pub(crate) struct ReleaseArtifactsCmd {
    #[command(subcommand)]
    command: ReleaseArtifactsCommand,
}

#[derive(clap::Subcommand)]
enum ReleaseArtifactsCommand {
    /// Install a family's verifiers and pilfflonk contracts from its build directory, and write
    /// its keys to a manifest for `render`.
    Install {
        /// Poseidon1, Poseidon2 or blake3.
        #[arg(long)]
        family: String,
        /// The family's build directory (provingKey/, provingKeySnark/).
        #[arg(long)]
        build_dir: PathBuf,
        /// Where to write the family's manifest.
        #[arg(long)]
        manifest_out: PathBuf,
        /// The build has no SNARK setup: keep this release's pilfflonk digest and contracts
        /// (requires the vadcop_final key to be unchanged).
        #[arg(long)]
        no_snark: bool,
        /// The repository root.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    /// Install `leaf.bin`, `compressed.bin` and, if present, `pilfflonk.bin` from `--proofs-dir`
    /// as the family's fixtures, with the guest's program VK.
    Fixtures {
        /// Poseidon1, Poseidon2 or blake3.
        #[arg(long)]
        family: String,
        /// Where `prove` wrote the proofs.
        #[arg(long)]
        proofs_dir: PathBuf,
        /// The guest ELF the proofs are of (its ROM setup, which `prove` runs).
        #[arg(long, required_unless_present = "program_vk", conflicts_with = "program_vk")]
        elf: Option<PathBuf>,
        /// Instead of `--elf`: the program VK the proofs commit to, as a JSON array of 4 u64s.
        #[arg(long)]
        program_vk: Option<PathBuf>,
        /// The repository root.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    /// Rewrite `verifier/src/keys.rs` from these manifests, keeping the other families' keys.
    Render {
        /// Manifests written by `install`.
        #[arg(long, num_args = 1..)]
        manifests: Vec<PathBuf>,
        /// The repository root.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
}

impl ReleaseArtifactsCmd {
    pub(crate) fn run(&self) -> Result<()> {
        match &self.command {
            ReleaseArtifactsCommand::Install {
                family,
                build_dir,
                manifest_out,
                no_snark,
                repo,
            } => install(family, build_dir, manifest_out, !no_snark, repo),
            ReleaseArtifactsCommand::Fixtures { family, proofs_dir, elf, program_vk: vk, repo } => {
                let vk = match (elf, vk) {
                    (Some(elf), _) => program_vk(elf, family)?,
                    (None, Some(path)) => read_verkey(path)?,
                    (None, None) => unreachable!("clap requires one"),
                };
                fixtures(family, proofs_dir, &vk, repo)
            }
            ReleaseArtifactsCommand::Render { manifests, repo } => render(manifests, repo),
        }
    }
}

fn family(hash: &str) -> Result<(&'static str, &'static str, &'static str)> {
    FAMILIES.into_iter().find(|(h, ..)| *h == hash).with_context(|| {
        format!("unknown family {hash:?}; expected Poseidon1, Poseidon2 or blake3")
    })
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn read_verkey(path: &Path) -> Result<[u64; 4]> {
    serde_json::from_value(read_json(path)?)
        .with_context(|| format!("{} is not a JSON array of 4 u64s", path.display()))
}

/// Without `snark`, the pilfflonk digest stays this release's, so the vadcop_final key it wraps
/// must be unchanged.
pub(crate) fn manifest_from_build(
    hash: &str,
    build_dir: &Path,
    snark: bool,
) -> Result<ReleaseManifest> {
    family(hash)?;
    let pk = build_dir.join("provingKey");
    let built = read_json(&pk.join("pilout.globalInfo.json"))?;
    let built = built.get("hash").and_then(|h| h.as_str()).unwrap_or("?");
    if !built.eq_ignore_ascii_case(hash) {
        bail!("{} is a {built} setup, not {hash}", build_dir.display());
    }
    let vadcop_final_vk =
        read_verkey(&stage_dir(build_dir, "vadcop_final").join("vadcop_final.verkey.json"))?;
    let vadcop_final_compressed_vk = read_verkey(
        &stage_dir(build_dir, "vadcop_final_compressed")
            .join("vadcop_final_compressed.verkey.json"),
    )?;
    if !snark {
        let release = zisk_verifier::vadcop_vk(hash, false);
        if release != Some(vadcop_final_vk) {
            bail!(
                "{hash}'s vadcop_final key is {vadcop_final_vk:?}, not this release's {release:?}: \
                 its pilfflonk wrap changed too, so set up the SNARK"
            );
        }
        return Ok(ReleaseManifest {
            hash: hash.to_string(),
            vadcop_final_vk,
            vadcop_final_compressed_vk,
            pilfflonk_vkey_digest: zisk_verifier::pilfflonk_vkey_digest(hash).map(str::to_owned),
            low_degree_test: LOW_DEGREE_TEST.to_string(),
        });
    }
    let vkey_path = pilfflonk_dir(build_dir).join("pilfflonk.vkey.json");
    if !vkey_path.exists() {
        bail!(
            "{} has no pilfflonk SNARK ({} is missing); the release's final SNARK is pilfflonk",
            build_dir.display(),
            vkey_path.display()
        );
    }
    let digest = read_json(&vkey_path)?
        .get("digest")
        .and_then(|d| d.as_str())
        .map(str::to_owned)
        .with_context(|| format!("{} has no digest", vkey_path.display()))?;
    ensure_digest(&digest)?;
    Ok(ReleaseManifest {
        hash: hash.to_string(),
        vadcop_final_vk,
        vadcop_final_compressed_vk,
        pilfflonk_vkey_digest: Some(digest),
        low_degree_test: LOW_DEGREE_TEST.to_string(),
    })
}

fn stage_dir(build_dir: &Path, stage: &str) -> PathBuf {
    build_dir.join("provingKey").join("zisk").join(stage)
}

fn snark_dir(build_dir: &Path) -> PathBuf {
    build_dir.join("provingKeySnark").join("final")
}

fn pilfflonk_dir(build_dir: &Path) -> PathBuf {
    snark_dir(build_dir).join("provingKey").join("final").join("pilfflonk")
}

/// Appended to every generated verifier: one proof length for `zisk-verifier`, whether the
/// generator emits `expected_proof_bytes` or (multiproof) `max_proof_bytes`.
const PROOF_SIZE_FN: &str = "
// Appended by `cargo-zisk-dev release-artifacts install`: the stage's proof length, from
// whichever function the generator emits.
pub fn proof_size() -> crate::ProofSize {
";

/// A generated verifier as `zisk-verifier` compiles it. Refuses one of another low-degree test.
pub(crate) fn rewrite_generated_verifier(src: &str, ldt: &str) -> Result<String> {
    let built = if src.contains("stark_verify_stir") { "STIR" } else { "FRI" };
    if built != ldt {
        bail!("the verifier checks a {built} proof; the release's is {ldt}");
    }
    if !src.contains("crate::") {
        bail!("not a generated verifier: it has no `crate::` paths to rewrite");
    }
    let size = if src.contains("pub fn max_proof_bytes()") {
        "crate::ProofSize::AtMost(max_proof_bytes())"
    } else if src.contains("pub fn expected_proof_bytes()") {
        "crate::ProofSize::Exact(expected_proof_bytes())"
    } else {
        bail!("the verifier exports neither expected_proof_bytes nor max_proof_bytes");
    };
    let mut out = src.replace("crate::", "proofman_verifier::");
    out.truncate(out.trim_end().len());
    out.push('\n');
    out.push_str(PROOF_SIZE_FN);
    out.push_str(&format!("    {size}\n}}\n"));
    Ok(out)
}

/// The generated `ZiskVerifier.sol`, importing the pilfflonk verifier from beside it.
pub(crate) fn rewrite_zisk_verifier_import(src: &str) -> Result<String> {
    const GENERATED_IMPORT: &str = "\"./provingKey/final/pilfflonk/pilfflonk.verifier.sol\"";
    if !src.contains(GENERATED_IMPORT) {
        bail!(
            "ZiskVerifier.sol does not import the pilfflonk verifier; is this a pilfflonk setup?"
        );
    }
    Ok(src.replace(GENERATED_IMPORT, "\"./PilfflonkVerifier.sol\""))
}

/// Every check runs before anything is written.
fn install(
    hash: &str,
    build_dir: &Path,
    manifest_out: &Path,
    snark: bool,
    repo: &Path,
) -> Result<()> {
    let (_, stem, _) = family(hash)?;
    let manifest = manifest_from_build(hash, build_dir, snark)?;

    let mut writes: Vec<(PathBuf, String)> = Vec::new();
    for (stage, ldt) in STAGES {
        let src = stage_dir(build_dir, stage).join(format!("{stage}.verifier.rs"));
        let text =
            std::fs::read_to_string(&src).with_context(|| format!("reading {}", src.display()))?;
        let text =
            rewrite_generated_verifier(&text, ldt).with_context(|| src.display().to_string())?;
        writes.push((repo.join("verifier/src").join(stem).join(format!("{stage}.rs")), text));
    }

    if snark {
        let snark = snark_dir(build_dir);
        let contracts = repo.join("zisk-contracts").join(stem);
        let zisk_verifier = std::fs::read_to_string(snark.join("ZiskVerifier.sol"))
            .with_context(|| format!("reading {}", snark.join("ZiskVerifier.sol").display()))?;
        writes.push((
            contracts.join("ZiskVerifier.sol"),
            rewrite_zisk_verifier_import(&zisk_verifier)?,
        ));
        for (from, to) in [
            (snark.join("IZiskVerifier.sol"), "IZiskVerifier.sol"),
            (pilfflonk_dir(build_dir).join("pilfflonk.verifier.sol"), "PilfflonkVerifier.sol"),
        ] {
            let text = std::fs::read_to_string(&from)
                .with_context(|| format!("reading {}", from.display()))?;
            writes.push((contracts.join(to), text));
        }
    }

    let mut manifest_json = serde_json::to_string_pretty(&manifest)?;
    manifest_json.push('\n');
    writes.push((manifest_out.to_path_buf(), manifest_json));

    for (path, text) in &writes {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

/// The program VK `elf` has under `hash`, from the ROM setup `prove` ran for it.
fn program_vk(elf: &Path, hash: &str) -> Result<[u64; 4]> {
    let elf = std::fs::read(elf).with_context(|| format!("reading {}", elf.display()))?;
    let mode: zisk_rom_setup::HashMode = hash.parse()?;
    let vk = zisk_rom_setup::rom_merkle_setup_verkey(&elf, &None, mode)?.vk;
    vk.try_into().map_err(|vk: Vec<u64>| anyhow::anyhow!("a {}-limb program VK", vk.len()))
}

fn load_proof(path: &Path, kind: ProofKind, hash: &str, program_vk: &[u64; 4]) -> Result<Proof> {
    let proof = Proof::load(path).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    if proof.kind() != kind || proof.get_program_vk().hash_mode.as_str() != hash {
        bail!(
            "{} is a {:?} proof of {}, not a {kind:?} proof of {hash}",
            path.display(),
            proof.kind(),
            proof.get_program_vk().hash_mode.as_str()
        );
    }
    if proof.get_program_vk().vk != program_vk {
        bail!("{} commits to another program than the ELF", path.display());
    }
    Ok(proof)
}

fn fixtures(hash: &str, proofs_dir: &Path, vk: &[u64; 4], repo: &Path) -> Result<()> {
    let (_, stem, _) = family(hash)?;
    let mut writes: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for (name, kind) in
        [("leaf", ProofKind::VadcopFinal), ("compressed", ProofKind::VadcopFinalMinimal)]
    {
        let proof = load_proof(&proofs_dir.join(format!("{name}.bin")), kind, hash, vk)?;
        let path = repo.join("verifier/tests/fixtures").join(format!("{stem}_{name}.bin"));
        writes.push((path, proof.to_bytes_without_seed_trailer()?));
    }
    let snark = proofs_dir.join("pilfflonk.bin");
    if snark.exists() {
        let proof = load_proof(&snark, ProofKind::Plonk, hash, vk)?;
        if !matches!(proof.body, ProofBody::Pilfflonk { .. }) {
            bail!("{} is not a pilfflonk proof", snark.display());
        }
        let path = repo.join("zisk-contracts/test").join(format!("{stem}_pilfflonk.bin"));
        writes.push((path, proof.to_bytes()?));
    }
    let path = repo.join("verifier/tests/fixtures").join(format!("{stem}_program_vk.json"));
    writes.push((path, format!("{}\n", serde_json::to_string(&vk)?).into_bytes()));

    for (path, bytes) in &writes {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

/// `0x` and 64 lowercase hex digits, as pilfflonk writes and compares it.
fn ensure_digest(digest: &str) -> Result<()> {
    let hex = digest.strip_prefix("0x").unwrap_or("");
    if hex.len() != 64 || !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        bail!("{digest:?} is not a pilfflonk digest (0x and 64 lowercase hex digits)");
    }
    Ok(())
}

/// The keys this binary was built with, as a manifest per family.
fn built_in_manifests() -> Vec<ReleaseManifest> {
    FAMILIES
        .iter()
        .filter_map(|(hash, ..)| {
            Some(ReleaseManifest {
                hash: hash.to_string(),
                vadcop_final_vk: zisk_verifier::vadcop_vk(hash, false)?,
                vadcop_final_compressed_vk: zisk_verifier::vadcop_vk(hash, true)?,
                pilfflonk_vkey_digest: zisk_verifier::pilfflonk_vkey_digest(hash)
                    .map(str::to_owned),
                low_degree_test: LOW_DEGREE_TEST.to_string(),
            })
        })
        .collect()
}

/// Every family's keys: `given`'s where it has them, else the built-in ones; in family order.
pub(crate) fn merge_manifests(
    built_in: Vec<ReleaseManifest>,
    given: Vec<ReleaseManifest>,
) -> Result<Vec<ReleaseManifest>> {
    for m in &given {
        family(&m.hash)?;
        if m.low_degree_test != LOW_DEGREE_TEST {
            bail!(
                "the {} manifest's compressed stage is {}, not {LOW_DEGREE_TEST}",
                m.hash,
                m.low_degree_test
            );
        }
        if let Some(digest) = &m.pilfflonk_vkey_digest {
            ensure_digest(digest)?;
        }
        if given.iter().filter(|o| o.hash == m.hash).count() > 1 {
            bail!("two manifests of {}", m.hash);
        }
    }
    Ok(FAMILIES
        .iter()
        .filter_map(|(hash, ..)| given.iter().chain(&built_in).find(|m| m.hash == *hash).cloned())
        .collect())
}

fn render(manifests: &[PathBuf], repo: &Path) -> Result<()> {
    let given = manifests
        .iter()
        .map(|path| {
            serde_json::from_value(read_json(path)?)
                .with_context(|| format!("{} is not a release manifest", path.display()))
        })
        .collect::<Result<Vec<ReleaseManifest>>>()?;
    let all = merge_manifests(built_in_manifests(), given)?;
    let path = repo.join("verifier/src/keys.rs");
    std::fs::write(&path, render_keys_rs(&all))
        .with_context(|| format!("writing {}", path.display()))
}

fn vk_const(name: &str, vk: &[u64; 4]) -> String {
    let limbs = vk.map(|l| l.to_string()).join(", ");
    format!("pub const {name}: [u64; VADCOP_VK_LEN_WORDS] =\n    [{limbs}];\n")
}

fn digest_const(name: &str, digest: Option<&str>) -> String {
    match digest {
        Some(d) => format!("pub const {name}: Option<&str> =\n    Some(\"{d}\");\n"),
        None => format!("pub const {name}: Option<&str> = None;\n"),
    }
}

pub(crate) fn render_keys_rs(manifests: &[ReleaseManifest]) -> String {
    let mut out = format!(
        "{GENERATED}

//! The keys of the release setup the verifiers in this crate were generated from.
//!
//! `VADCOP_FINAL_*`: the `<stage>.verkey.json` of each family's proving key. A key is the root of
//! a stage's constant tree. It is shared by every ZisK program and identifies the prover, not the
//! program: check `Verified::program_vk` for that. It covers leaf proofs only; a recurser proof
//! verifies under its recurser's own key.
//!
//! `PILFFLONK_VKEY_DIGEST_*`: the `digest` of each family's `pilfflonk.vkey.json`, which pins the
//! key (see [`pilfflonk_vkey_digest`](crate::pilfflonk_vkey_digest)).
//!
//! Regenerate with `tools/release-artifacts/regen.sh`, together with the verifiers.

use crate::VADCOP_VK_LEN_WORDS;
"
    );
    for m in manifests {
        let suffix = FAMILIES.iter().find(|(h, ..)| *h == m.hash).map_or("", |(.., s)| s);
        out.push('\n');
        out.push_str(&vk_const(&format!("VADCOP_FINAL_VK_{suffix}"), &m.vadcop_final_vk));
        out.push('\n');
        out.push_str(&vk_const(
            &format!("VADCOP_FINAL_COMPRESSED_VK_{suffix}"),
            &m.vadcop_final_compressed_vk,
        ));
        out.push('\n');
        out.push_str(&digest_const(
            &format!("PILFFLONK_VKEY_DIGEST_{suffix}"),
            m.pilfflonk_vkey_digest.as_deref(),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(hash: &str, seed: u64, digest: Option<&str>) -> ReleaseManifest {
        ReleaseManifest {
            hash: hash.to_string(),
            vadcop_final_vk: [seed, seed + 1, seed + 2, seed + 3],
            vadcop_final_compressed_vk: [seed + 4, seed + 5, seed + 6, seed + 7],
            pilfflonk_vkey_digest: digest.map(str::to_owned),
            low_degree_test: LOW_DEGREE_TEST.to_string(),
        }
    }

    /// keys.rs is a fixed point: rendering the keys it compiled to writes it back unchanged.
    /// Read at run time: a packaged crate has no sibling crates, and there the test passes.
    #[test]
    fn rendering_the_built_in_keys_writes_keys_rs_back() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../verifier/src/keys.rs");
        let Ok(committed) = std::fs::read_to_string(path) else { return };
        let built_in = built_in_manifests();
        assert_eq!(built_in.len(), 3);
        assert_eq!(render_keys_rs(&merge_manifests(built_in, vec![]).unwrap()), committed);
    }

    #[test]
    fn a_given_family_replaces_only_its_own_keys() {
        let built_in = vec![
            manifest("Poseidon1", 10, None),
            manifest("Poseidon2", 20, None),
            manifest("blake3", 30, Some("0xcc")),
        ];
        let merged = merge_manifests(
            built_in.clone(),
            vec![manifest(
                "Poseidon2",
                99,
                Some("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            )],
        )
        .unwrap();
        assert_eq!(
            merged,
            vec![
                built_in[0].clone(),
                manifest(
                    "Poseidon2",
                    99,
                    Some("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
                ),
                built_in[2].clone()
            ]
        );
    }

    #[test]
    fn a_manifest_is_checked_before_it_is_rendered() {
        let mut fri = manifest("blake3", 1, None);
        fri.low_degree_test = "FRI".to_string();
        assert!(merge_manifests(vec![], vec![fri]).is_err());
        let digest = format!("0x{}", "ab".repeat(32));
        assert!(merge_manifests(vec![], vec![manifest("blake3", 1, Some(&digest))]).is_ok());
        for bad in ["0xAB", "0xcc", &digest.to_uppercase(), "ab\"cd"] {
            assert!(
                merge_manifests(vec![], vec![manifest("blake3", 1, Some(bad))]).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn two_manifests_of_one_family_are_refused() {
        let given = vec![manifest("blake3", 1, None), manifest("blake3", 2, None)];
        assert!(merge_manifests(vec![], given).is_err());
    }

    #[test]
    fn keys_rs_carries_each_digest_or_none() {
        let out =
            render_keys_rs(&[manifest("Poseidon1", 1, None), manifest("blake3", 2, Some("0xcc"))]);
        assert!(
            out.contains("pub const PILFFLONK_VKEY_DIGEST_POSEIDON1: Option<&str> = None;"),
            "{out}"
        );
        assert!(
            out.contains(
                "pub const PILFFLONK_VKEY_DIGEST_BLAKE3: Option<&str> =\n    Some(\"0xcc\");"
            ),
            "{out}"
        );
    }

    #[test]
    fn a_manifest_round_trips() {
        let m = manifest("Poseidon2", 7, Some("0xdd"));
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"vadcopFinalCompressedVk\""), "{json}");
        assert_eq!(serde_json::from_str::<ReleaseManifest>(&json).unwrap(), m);
    }

    #[test]
    fn the_rewrite_points_the_generated_verifier_at_proofman_verifier() {
        let src = "use crate::{stark_verify, Boundary, FriEvalGroup, FriEvalRef, VerifierInfo};\n\
                   use crate::VadcopFinalProof;\n\
                   pub fn expected_proof_bytes() -> usize {\n\
                   crate::expected_proof_size_bytes(&verifier_info())\n}\n";
        let out = rewrite_generated_verifier(src, "FRI").unwrap();
        let (generated, appended) = out.split_once(PROOF_SIZE_FN).unwrap();
        assert!(!generated.contains("crate::"));
        assert_eq!(generated.matches("proofman_verifier::").count(), 3);
        assert_eq!(appended, "    crate::ProofSize::Exact(expected_proof_bytes())\n}\n");
    }

    #[test]
    fn a_multiproof_verifier_admits_at_most_its_max() {
        let src = "use crate::stark_verify_stir;\npub fn max_proof_bytes() -> usize {\n\
                   crate::expected_stir_proof_size_bytes(&verifier_info()) + 8\n}\n";
        let out = rewrite_generated_verifier(src, "STIR").unwrap();
        assert!(out.ends_with("    crate::ProofSize::AtMost(max_proof_bytes())\n}\n"), "{out}");
    }

    #[test]
    fn a_verifier_of_another_low_degree_test_is_refused() {
        let src = "use crate::stark_verify;\npub fn expected_proof_bytes() -> usize { 8 }\n";
        let err = rewrite_generated_verifier(src, "STIR").unwrap_err();
        assert!(err.to_string().contains("FRI") && err.to_string().contains("STIR"), "{err}");
        let src = "use crate::stark_verify_stir;\npub fn expected_proof_bytes() -> usize { 8 }\n";
        assert!(rewrite_generated_verifier(src, "FRI").is_err());
    }

    #[test]
    fn a_source_with_nothing_to_rewrite_is_refused() {
        let src = "use proofman_verifier::VadcopFinalProof;\npub fn expected_proof_bytes() {}";
        assert!(rewrite_generated_verifier(src, "FRI").is_err());
        assert!(rewrite_generated_verifier("use crate::X;", "FRI").is_err());
    }

    #[test]
    fn the_zisk_verifier_imports_its_sibling_pilfflonk_verifier() {
        let src = "import {PilfflonkVerifier} from \"./provingKey/final/pilfflonk/pilfflonk.verifier.sol\";";
        assert_eq!(
            rewrite_zisk_verifier_import(src).unwrap(),
            "import {PilfflonkVerifier} from \"./PilfflonkVerifier.sol\";"
        );
        assert!(rewrite_zisk_verifier_import(
            "import {PlonkVerifier} from \"./PlonkVerifier.sol\";"
        )
        .is_err());
    }

    const FRI_VERIFIER: &str = "use crate::stark_verify;\npub fn expected_proof_bytes() {}\n";
    const STIR_MULTIPROOF_VERIFIER: &str =
        "use crate::stark_verify_stir;\npub fn max_proof_bytes() {}\n";

    /// A fake setup of `hash` in `dir`, with or without its pilfflonk key.
    fn fake_build(dir: &Path, hash: &str, pilfflonk: bool) {
        let pk = dir.join("provingKey");
        std::fs::create_dir_all(&pk).unwrap();
        std::fs::write(pk.join("pilout.globalInfo.json"), format!("{{\"hash\":\"{hash}\"}}"))
            .unwrap();
        for (stage, vk, verifier) in [
            ("vadcop_final", "[1,2,3,4]", FRI_VERIFIER),
            ("vadcop_final_compressed", "[5,6,7,8]", STIR_MULTIPROOF_VERIFIER),
        ] {
            let d = stage_dir(dir, stage);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(format!("{stage}.verkey.json")), vk).unwrap();
            std::fs::write(d.join(format!("{stage}.verifier.rs")), verifier).unwrap();
        }
        let snark = snark_dir(dir);
        std::fs::create_dir_all(&snark).unwrap();
        std::fs::write(
            snark.join("ZiskVerifier.sol"),
            "import {PilfflonkVerifier} from \"./provingKey/final/pilfflonk/pilfflonk.verifier.sol\";",
        )
        .unwrap();
        std::fs::write(snark.join("IZiskVerifier.sol"), "interface IZiskVerifier {}").unwrap();
        if pilfflonk {
            let p = pilfflonk_dir(dir);
            std::fs::create_dir_all(&p).unwrap();
            std::fs::write(p.join("pilfflonk.vkey.json"), "{\"digest\":\"0xabababababababababababababababababababababababababababababababab\"}").unwrap();
            std::fs::write(p.join("pilfflonk.verifier.sol"), "contract PilfflonkVerifier {}")
                .unwrap();
        }
    }

    #[test]
    fn install_writes_the_family_and_render_its_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let (build, repo) = (tmp.path().join("build"), tmp.path().join("repo"));
        let manifest_out = tmp.path().join("poseidon2.json");
        fake_build(&build, "Poseidon2", true);
        std::fs::create_dir_all(repo.join("verifier/src")).unwrap();

        install("Poseidon2", &build, &manifest_out, true, &repo).unwrap();
        let verifier =
            std::fs::read_to_string(repo.join("verifier/src/poseidon2/vadcop_final.rs")).unwrap();
        assert!(verifier.starts_with("use proofman_verifier::stark_verify;\n"), "{verifier}");
        assert!(verifier.ends_with("ProofSize::Exact(expected_proof_bytes())\n}\n"), "{verifier}");
        let compressed =
            std::fs::read_to_string(repo.join("verifier/src/poseidon2/vadcop_final_compressed.rs"))
                .unwrap();
        assert!(compressed.ends_with("ProofSize::AtMost(max_proof_bytes())\n}\n"), "{compressed}");
        let sol = std::fs::read_to_string(repo.join("zisk-contracts/poseidon2/ZiskVerifier.sol"))
            .unwrap();
        assert!(sol.contains("\"./PilfflonkVerifier.sol\""), "{sol}");
        assert!(repo.join("zisk-contracts/poseidon2/PilfflonkVerifier.sol").exists());

        render(&[manifest_out], &repo).unwrap();
        let keys = std::fs::read_to_string(repo.join("verifier/src/keys.rs")).unwrap();
        assert!(keys.contains("VADCOP_FINAL_COMPRESSED_VK_POSEIDON2: [u64; VADCOP_VK_LEN_WORDS] =\n    [5, 6, 7, 8];"), "{keys}");
        assert!(
            keys.contains(
                "Some(\"0xabababababababababababababababababababababababababababababababab\")"
            ),
            "{keys}"
        );
        let p1 =
            zisk_verifier::vadcop_vk("Poseidon1", false).unwrap().map(|l| l.to_string()).join(", ");
        assert!(keys.contains(&p1), "{keys}");
    }

    #[test]
    fn a_build_without_its_snark_keeps_the_release_pilfflonk_key() {
        let tmp = tempfile::tempdir().unwrap();
        let (build, repo) = (tmp.path().join("build"), tmp.path().join("repo"));
        fake_build(&build, "blake3", false);
        std::fs::create_dir_all(repo.join("verifier/src")).unwrap();
        let err = manifest_from_build("blake3", &build, false).unwrap_err().to_string();
        assert!(err.contains("set up the SNARK"), "{err}");

        let release = zisk_verifier::vadcop_vk("blake3", false).unwrap();
        std::fs::write(
            stage_dir(&build, "vadcop_final").join("vadcop_final.verkey.json"),
            serde_json::to_string(&release).unwrap(),
        )
        .unwrap();
        let manifest_out = tmp.path().join("blake3.json");
        install("blake3", &build, &manifest_out, false, &repo).unwrap();
        let m: ReleaseManifest =
            serde_json::from_str(&std::fs::read_to_string(&manifest_out).unwrap()).unwrap();
        assert_eq!(m.vadcop_final_vk, release);
        assert_eq!(
            m.pilfflonk_vkey_digest.as_deref(),
            zisk_verifier::pilfflonk_vkey_digest("blake3")
        );
        assert!(!repo.join("zisk-contracts").exists());
    }

    #[test]
    fn a_build_of_another_family_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        fake_build(tmp.path(), "blake3", true);
        let err = manifest_from_build("Poseidon2", tmp.path(), true).unwrap_err().to_string();
        assert!(err.contains("blake3") && err.contains("Poseidon2"), "{err}");
    }

    #[test]
    fn install_writes_nothing_when_a_check_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let (build, repo) = (tmp.path().join("build"), tmp.path().join("repo"));
        let manifest_out = tmp.path().join("poseidon1.json");
        fake_build(&build, "Poseidon1", false);
        std::fs::create_dir_all(&repo).unwrap();

        let err = install("Poseidon1", &build, &manifest_out, true, &repo).unwrap_err().to_string();
        assert!(err.contains("pilfflonk"), "{err}");
        assert!(!manifest_out.exists());
        assert!(!repo.join("verifier").exists());
    }
}
