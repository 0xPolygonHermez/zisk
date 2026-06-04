//! Bakes two fingerprints of the *source* inputs behind the on-disk caches, so
//! a change forces regeneration (the caches are keyed by filename, and none of
//! these inputs has a runtime-hashable compiled artifact).
//!
//! `ZISK_ROM_INPUTS_HASH` covers the transpiler (`zisk-core`) plus the float ELF
//! `elf2rom` embeds, and keys the ROM/verkey cache (`utils.rs`).
//! `ZISK_ASM_INPUTS_HASH` covers the ROM inputs plus `emulator-asm/src`, and
//! keys the asm-binary cache (`asm_setup.rs`) alongside the linked libs hashed
//! at runtime.
//!
//! The ROM inputs are a subset of the asm inputs; keeping them separate avoids
//! regenerating the expensive ROM/verkey on an `emulator-asm`-only change.

use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // rom-setup lives at <workspace>/rom-setup.
    let ws = manifest.parent().expect("rom-setup has a parent dir");

    let core_src = ws.join("core/src");
    let emu_src = ws.join("emulator-asm/src");

    // We `rerun-if-changed` each existing file below, not the dirs (cargo's
    // directory watch doesn't reliably fire on file *adds*). That's sufficient:
    // a new source file only affects the output once it's referenced — a `mod`
    // in `lib.rs`, an entry in `emulator-asm/Makefile`, or an `.asm`/`.inc`
    // include — and editing that referencing file (which IS watched) triggers
    // the rerun that picks the new file up.

    // Deny-list (fail-safe): `zisk-core` files that don't affect the transpiled
    // ROM — the Rust-emulator runtime and CLI binaries. Anything else is hashed
    // by default, so a new transpiler module is covered automatically.
    let rom_excluded =
        [ws.join("core/src/bin"), core_src.join("inst_context.rs"), core_src.join("mem.rs")];
    let mut rom_files = collect_files(&core_src, &rom_excluded);
    rom_files.push(ws.join("lib-float/c/lib/ziskfloat.elf")); // embedded into the ROM
    let rom_hash = hash_files(blake3::Hasher::new(), ws, rom_files);

    let mut asm_files = collect_files(&emu_src, &[]);
    asm_files.push(ws.join("emulator-asm/Makefile"));
    let mut seeded = blake3::Hasher::new();
    seeded.update(rom_hash.as_bytes());
    seeded.update(&[0xffu8]);
    let asm_hash = hash_files(seeded, ws, asm_files);

    println!("cargo:rustc-env=ZISK_ROM_INPUTS_HASH={rom_hash}");
    println!("cargo:rustc-env=ZISK_ASM_INPUTS_HASH={asm_hash}");
}

/// Fold a list of files (workspace-relative path + content) into `hasher` and
/// return a 12-hex digest, emitting `rerun-if-changed` for each.
fn hash_files(mut hasher: blake3::Hasher, ws: &Path, mut files: Vec<PathBuf>) -> String {
    files.sort(); // deterministic, independent of readdir order
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let rel = f.strip_prefix(ws).unwrap_or(f);
        hasher.update(rel.to_string_lossy().as_bytes());
        hasher.update(&[0u8]);
        if let Ok(bytes) = std::fs::read(f) {
            hasher.update(&(bytes.len() as u64).to_le_bytes());
            hasher.update(&bytes);
        }
        hasher.update(&[0xffu8]);
    }
    hasher.finalize().to_hex().as_str()[..12].to_string()
}

/// Recursively collect files under `dir` (no-op if absent), skipping anything
/// matched by `excluded` (an exact path or an ancestor dir).
fn collect_files(dir: &Path, excluded: &[PathBuf]) -> Vec<PathBuf> {
    let is_excluded = |p: &Path| excluded.iter().any(|e| p == e || p.starts_with(e));
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if is_excluded(&path) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out
}
