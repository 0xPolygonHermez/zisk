use anyhow::{Context, Result};
use proofman_fields::{Goldilocks, PrimeField64};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use zisk_common::{ZiskPaths, PROGRAM_VK_LEN};
use zisk_core::{MAX_INPUT_SIZE, RAM_SIZE};
use zisk_pil::{RomRomTrace, PILOUT_HASH};
use zisk_sm_rom::CustomRom;

pub use zisk_common::HashMode;

/// Resolve the artifact output directory, defaulting to the ZisK cache when
/// `output_dir` is `None`. Creates the directory if needed and returns its
/// canonical absolute path.
pub fn get_output_path(output_dir: &Option<PathBuf>) -> Result<PathBuf> {
    let output_path = if output_dir.is_none() {
        let cache_path = ZiskPaths::global().cache.clone();
        ensure_dir_exists(&cache_path);
        cache_path
    } else {
        ensure_dir_exists(output_dir.as_ref().unwrap());
        output_dir.clone().unwrap()
    };

    let output_path = fs::canonicalize(&output_path)
        .with_context(|| format!("Failed to get absolute path for {output_path:?}"))?;

    Ok(output_path)
}

/// blake3 digest of a ROM custom-commit trace: the exact data the verkey commits to.
type RomDigest = [u8; 32];

/// Digests of the ROM traces built in this process, by ELF hash. Building one transpiles the
/// ELF, so each ELF's is computed once.
static DIGESTS: LazyLock<Mutex<HashMap<String, RomDigest>>> = LazyLock::new(Default::default);

/// The digest of `elf`'s ROM trace, if this process has built it.
pub(crate) fn cached_rom_digest(elf: &[u8]) -> Option<RomDigest> {
    DIGESTS.lock().unwrap().get(&get_elf_data_hash(elf)).copied()
}

/// Build `elf`'s ROM trace and its digest (over the canonical values of its elements).
pub(crate) fn rom_trace_with_digest<F: PrimeField64>(
    elf: &[u8],
) -> Result<(RomRomTrace<F>, RomDigest)> {
    let mut trace = CustomRom::build::<F>(elf).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let buffer = trace.get_buffer::<F>();
    let mut hasher = blake3::Hasher::new();
    for chunk in buffer.chunks(1 << 13) {
        let bytes: Vec<u8> =
            chunk.iter().flat_map(|x| x.as_canonical_u64().to_le_bytes()).collect();
        hasher.update(&bytes);
    }
    let digest = *hasher.finalize().as_bytes();
    DIGESTS.lock().unwrap().insert(get_elf_data_hash(elf), digest);
    let trace = RomRomTrace::new_from_vec(buffer).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok((trace, digest))
}

fn rom_digest(elf: &[u8]) -> Result<RomDigest> {
    match cached_rom_digest(elf) {
        Some(digest) => Ok(digest),
        None => Ok(rom_trace_with_digest::<Goldilocks>(elf)?.1),
    }
}

/// Cache key of `elf`'s ROM custom commit and verkey under `hash_mode`.
///
/// Derived from the ROM trace itself (so from the ELF, the transpiler and the trace layout,
/// padding included), the PIL and the commit parameters: a change to any of them is a cache
/// miss rather than a stale verkey.
pub fn rom_key(elf: &[u8], hash_mode: HashMode) -> Result<String> {
    Ok(rom_cache_key(&rom_digest(elf)?, hash_mode))
}

/// Check that `key` has the form of a [`rom_key`] (32 lowercase hex digits), so one received
/// from elsewhere is safe to put in a file name.
pub fn check_rom_key(key: &str) -> Result<()> {
    if key.len() == 32 && key.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        Ok(())
    } else {
        anyhow::bail!("invalid ROM key {key:?}")
    }
}

pub(crate) fn rom_cache_key(digest: &RomDigest, hash_mode: HashMode) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(digest);
    hasher.update(PILOUT_HASH.as_bytes());
    hasher.update(hash_mode.file_tag().as_bytes());
    hasher.update(&hash_mode.blowup_factor().to_le_bytes());
    hasher.update(&hash_mode.merkle_tree_arity().to_le_bytes());
    hasher.finalize().to_hex()[..32].to_string()
}

/// Read a verification key (a `PROGRAM_VK_LEN` array of little-endian `u64`s)
/// from `verkey_path`. Returns `None` if the file does not exist.
pub fn get_elf_vk(verkey_path: &Path) -> Result<Option<Vec<u64>>> {
    if !verkey_path.exists() {
        return Ok(None);
    }

    let mut file = File::open(verkey_path)?;
    let mut vk = vec![0u64; PROGRAM_VK_LEN];
    for word in vk.iter_mut() {
        let mut buf = [0u8; 8];
        file.read_exact(&mut buf)?;
        *word = u64::from_le_bytes(buf);
    }
    Ok(Some(vk))
}

/// Read the ELF at `elf_path` and return the hex-encoded blake3 hash of its
/// bytes. See [`get_elf_data_hash`] for the in-memory variant.
pub fn get_elf_data_hash_from_path(elf_path: &Path) -> Result<String> {
    let elf_data =
        fs::read(elf_path).with_context(|| format!("Error reading ELF file: {elf_path:?}"))?;

    let hash = blake3::hash(&elf_data).to_hex().to_string();

    Ok(hash)
}

/// The hex-encoded blake3 hash of `elf`. This is the content address used to
/// name and locate all of a program's cached artifacts.
pub fn get_elf_data_hash(elf: &[u8]) -> String {
    blake3::hash(elf).to_hex().to_string()
}

/// Tag of the memory the executors reserve (the `input_2gb` and `ram_2gb` features of
/// zisk-core), appended to the hash in the names of the assembly emulator binaries, which reserve
/// that memory. The ROM, and so its commitment and verkey, does not depend on it. Every
/// configuration has its own tag, the default one included, so a cached binary is never taken
/// for one that reserves other sizes.
pub fn mem_config_tag() -> String {
    format!("_in{}m_ram{}m", MAX_INPUT_SIZE >> 20, RAM_SIZE >> 20)
}

/// File stem of a program's ROM artifacts: `{elf_hash}_{mode}_{rom_key}`. The ELF hash
/// leads so `clean-cache --elf` finds them; the mode is there for whoever reads the name.
fn rom_file_stem(elf_hash: &str, rom_key: &str, hash_mode: HashMode) -> String {
    format!("{elf_hash}_{}_{rom_key}", hash_mode.file_tag())
}

/// Cache path of the ROM custom-commit binary (`_gpu` for the GPU layout).
pub fn get_rom_bin_file_path(
    cache_dir: &Path,
    elf_hash: &str,
    rom_key: &str,
    hash_mode: HashMode,
    gpu: bool,
) -> PathBuf {
    let gpu_suffix = if gpu { "_gpu" } else { "" };
    cache_dir.join(format!("{}{gpu_suffix}.bin", rom_file_stem(elf_hash, rom_key, hash_mode)))
}

/// Cache path of the verkey file, shared by the CPU and GPU layouts.
pub fn get_rom_verkey_file_path(
    cache_dir: &Path,
    elf_hash: &str,
    rom_key: &str,
    hash_mode: HashMode,
) -> PathBuf {
    cache_dir.join(format!("{}.verkey.bin", rom_file_stem(elf_hash, rom_key, hash_mode)))
}

/// Create `path` (and any missing parents) if it does not already exist.
///
/// # Panics
/// Panics if the directory cannot be created for a reason other than it
/// already existing.
pub fn ensure_dir_exists(path: &Path) {
    if let Err(e) = std::fs::create_dir_all(path) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            panic!("Failed to create cache directory {path:?}: {e}");
        }
    }
}
