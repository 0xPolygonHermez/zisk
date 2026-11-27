use anyhow::{Context, Result};
use proofman_common::{write_custom_commit_trace, ProofCtx};
use proofman_fields::{Goldilocks, PrimeField64};
use std::path::{Path, PathBuf};
use zisk_common::ProgramVK;
use zisk_pil::RomRomTrace;
use zisk_sm_rom::CustomRom;

use crate::{cached_rom_digest, rom_cache_key, rom_trace_with_digest};
use crate::{
    get_elf_data_hash, get_elf_vk, get_output_path, get_rom_bin_file_path,
    get_rom_verkey_file_path, rom_key, HashMode,
};

/// A program's ROM setup.
#[derive(Debug, Clone)]
pub struct RomSetup {
    /// The program verification key.
    pub vk: ProgramVK,
    /// The cache key its ROM artifacts are stored under (see [`rom_key`]).
    pub rom_key: String,
}

fn validate_custom_commit_file_size(elf_bin_path: &Path, hash_mode: HashMode) -> Result<()> {
    let n = RomRomTrace::<Goldilocks>::NUM_ROWS as u64;
    let n_cols = RomRomTrace::<Goldilocks>::ROW_SIZE as u64;
    let n_extended = hash_mode.blowup_factor() * n;
    proofman_common::custom_commit_words_per_row(
        elf_bin_path,
        n,
        n_extended,
        n_cols,
        hash_mode.merkle_tree_arity(),
    )
    .map(|_| ())
    .map_err(|e| {
        anyhow::anyhow!("cached ROM custom commit '{}' is unusable: {e}", elf_bin_path.display())
    })
}

/// Resolve the path of the cached ROM custom-commit binary for `elf_hash` under `rom_key`.
///
/// Errors if the ROM binary or its verkey file has not been generated yet (run
/// [`rom_merkle_setup`] first).
pub fn get_rom_path<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf_hash: &str,
    rom_key: &str,
    output_dir: &Option<PathBuf>,
    hash_mode: HashMode,
) -> Result<PathBuf> {
    let output_path = get_output_path(output_dir)?;

    let elf_bin_path = get_rom_bin_file_path(&output_path, elf_hash, rom_key, hash_mode, pctx.gpu);
    let elf_verkey_bin_path = get_rom_verkey_file_path(&output_path, elf_hash, rom_key, hash_mode);

    if !elf_bin_path.exists() || !elf_verkey_bin_path.exists() {
        return Err(anyhow::anyhow!(
            "ROM files not found for ELF hash {}. Expected paths: {:?} and {:?}",
            elf_hash,
            elf_bin_path,
            elf_verkey_bin_path
        ));
    }

    Ok(elf_bin_path)
}

/// Perform the ROM Merkle setup for `elf` and return its verkey and cache key.
///
/// Builds the ROM custom commit and derives the verkey from its Merkle root, writing both to
/// the cache. When `force` is false and valid cached artifacts already exist, the cached
/// verkey is returned instead of regenerating.
pub fn rom_merkle_setup<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    elf: &[u8],
    output_dir: &Option<PathBuf>,
    force: bool,
    hash_mode: HashMode,
) -> Result<RomSetup> {
    let output_path = get_output_path(output_dir)?;

    let elf_hash = get_elf_data_hash(elf);
    // The key needs the trace's digest; keep the trace if it is built for it, for a miss.
    let (trace, digest) = match cached_rom_digest(elf) {
        Some(digest) => (None, digest),
        None => {
            let (trace, digest) = rom_trace_with_digest::<F>(elf)?;
            (Some(trace), digest)
        }
    };
    let rom_key = rom_cache_key(&digest, hash_mode);

    let elf_bin_path =
        get_rom_bin_file_path(&output_path, &elf_hash, &rom_key, hash_mode, pctx.gpu);
    let elf_verkey_bin_path =
        get_rom_verkey_file_path(&output_path, &elf_hash, &rom_key, hash_mode);

    // A cached ROM that no longer matches the current layout (one written before the packed
    // custom-commit format, say) is a cache MISS, not an error: fall through and rebuild it.
    let reuse_cached = !force
        && elf_bin_path.exists()
        && elf_verkey_bin_path.exists()
        && match validate_custom_commit_file_size(&elf_bin_path, hash_mode) {
            Ok(()) => true,
            Err(err) => {
                tracing::info!("Regenerating ROM custom commit: {err}");
                false
            }
        };

    if reuse_cached {
        tracing::info!("Using cached ROM setup {}", elf_bin_path.display());
        let vk = get_elf_vk(elf_verkey_bin_path.as_path())?
            .ok_or_else(|| anyhow::anyhow!("Failed to read existing verkey file"))?;
        return Ok(RomSetup { vk: ProgramVK { vk, hash_mode }, rom_key });
    }

    tracing::info!("Generating ROM setup {}", elf_bin_path.display());
    let mut trace = match trace {
        Some(trace) => trace,
        None => CustomRom::build::<F>(elf).map_err(|e| anyhow::anyhow!(e.to_string()))?,
    };
    let root = write_custom_commit_trace(
        pctx,
        &mut trace,
        hash_mode.blowup_factor(),
        hash_mode.merkle_tree_arity(),
        &elf_bin_path,
    )?;

    tracing::info!("Root hash: {:?}", root);

    let vk: Vec<u64> = root.iter().map(|x| x.as_canonical_u64()).collect();

    let vk_bytes: Vec<u8> = vk.iter().flat_map(|w| w.to_le_bytes()).collect();
    std::fs::write(&elf_verkey_bin_path, &vk_bytes)?;

    Ok(RomSetup { vk: ProgramVK { vk, hash_mode }, rom_key })
}

/// Read the cached verkey for `elf_hash` under `rom_key`. `Ok(None)` when there is none; one
/// that exists and will not read stays an error, so callers probing modes cannot read it as
/// absence.
pub fn read_cached_verkey(
    elf_hash: &str,
    rom_key: &str,
    output_dir: &Option<PathBuf>,
    hash_mode: HashMode,
) -> Result<Option<ProgramVK>> {
    let output_path = get_output_path(output_dir)?;
    let path = get_rom_verkey_file_path(&output_path, elf_hash, rom_key, hash_mode);

    if !path.exists() {
        return Ok(None);
    }

    let vk = get_elf_vk(&path)
        .with_context(|| format!("Failed to read verkey at {}", path.display()))?
        .ok_or_else(|| anyhow::anyhow!("Empty verkey file at {}", path.display()))?;

    Ok(Some(ProgramVK { vk, hash_mode }))
}

/// The error for a verkey not found under `rom_keys`, the keys this build computes. Verkeys
/// cached for the same ELF under other keys are listed: they belong to another hash mode, or
/// to a ZisK version whose ROM differs, and are not the one asked for.
pub fn missing_verkey_error(
    elf_hash: &str,
    rom_keys: &[String],
    output_dir: &Option<PathBuf>,
) -> anyhow::Error {
    let prefix = format!("{elf_hash}_");
    let mut others: Vec<PathBuf> = get_output_path(output_dir)
        .and_then(|dir| Ok(std::fs::read_dir(dir)?))
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".verkey.bin"))
                })
                .collect()
        })
        .unwrap_or_default();
    others.sort();
    let keys = rom_keys.join(", ");
    if others.is_empty() {
        anyhow::anyhow!("ROM setup has not been performed yet for ELF {elf_hash} (ROM key {keys})")
    } else {
        anyhow::anyhow!(
            "no verkey for ELF {elf_hash} under ROM key {keys}; the cache has it only under \
             other keys ({}), from another hash mode or a ZisK version whose ROM differs. Run \
             setup for this one",
            others.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
        )
    }
}

/// Read the program verification key for `elf` from the cache.
///
/// Unlike [`rom_merkle_setup`], this never regenerates: it errors if the verkey
/// file does not already exist.
pub fn rom_merkle_setup_verkey(
    elf: &[u8],
    output_dir: &Option<PathBuf>,
    hash_mode: HashMode,
) -> Result<ProgramVK> {
    let elf_hash = get_elf_data_hash(elf);
    let rom_key = rom_key(elf, hash_mode)?;
    read_cached_verkey(&elf_hash, &rom_key, output_dir, hash_mode)?
        .ok_or_else(|| missing_verkey_error(&elf_hash, std::slice::from_ref(&rom_key), output_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rom_key_tracks_its_inputs() {
        let a = crate::rom_cache_key(&[1; 32], HashMode::Poseidon2);
        assert_eq!(a, crate::rom_cache_key(&[1; 32], HashMode::Poseidon2));
        assert_ne!(a, crate::rom_cache_key(&[2; 32], HashMode::Poseidon2));
        assert_ne!(a, crate::rom_cache_key(&[1; 32], HashMode::Poseidon1));
        crate::check_rom_key(&a).unwrap();
        assert!(crate::check_rom_key("../../etc/passwd").is_err());
        assert!(crate::check_rom_key(&a.to_uppercase()).is_err());
    }

    #[test]
    fn missing_verkey_names_a_stale_one() {
        let dir = std::env::temp_dir().join(format!("rom-setup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = Some(dir.clone());
        let keys = ["newkey".to_string()];
        let err = missing_verkey_error("abc", &keys, &out).to_string();
        assert!(err.contains("has not been performed") && err.contains("newkey"), "{err}");

        let stale = get_rom_verkey_file_path(&dir, "abc", "oldkey", HashMode::Poseidon2);
        std::fs::write(&stale, [0u8; 32]).unwrap();
        let err = missing_verkey_error("abc", &keys, &out).to_string();
        assert!(err.contains("oldkey") && err.contains("other keys"), "{err}");
        assert!(read_cached_verkey("abc", "newkey", &out, HashMode::Poseidon2).unwrap().is_none());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
