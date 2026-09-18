//! Compiles the GPU Keccakf witness kernel when CUDA is available.
//!
//! Sets `cfg(gpu)`; everything GPU-side in this crate is gated on it, so the
//! crate builds unchanged on hosts without nvcc.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Locates `nvcc` as an ABSOLUTE path. `cuda_lib_dir` derives `lib64` from it, so a
/// bare `"nvcc"` would not canonicalize (no PATH lookup in `Path::canonicalize`) and
/// would silently fall back to `/usr/local/cuda/lib64` — wrong on an `/opt/cuda` host,
/// where the link then fails with `cannot find -lcudart`.
fn find_nvcc() -> Option<PathBuf> {
    if cfg!(feature = "cpu-only") || cfg!(target_os = "macos") {
        return None;
    }
    let from_path = env::var_os("PATH").into_iter().flat_map(|paths| {
        env::split_paths(&paths).map(|dir| dir.join("nvcc")).collect::<Vec<_>>()
    });
    from_path
        .chain(["/usr/local/cuda/bin/nvcc", "/opt/cuda/bin/nvcc"].iter().map(PathBuf::from))
        .find(|candidate| {
            candidate.is_file()
                && Command::new(candidate)
                    .arg("--version")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
        })
}

/// `<prefix>/bin/nvcc` -> `<prefix>/lib64`, following symlinks (the usual
/// `/usr/local/cuda` -> `/usr/local/cuda-12.9` indirection).
fn cuda_lib_dir(nvcc: &Path) -> PathBuf {
    nvcc.canonicalize()
        .ok()
        .and_then(|abs| abs.parent().and_then(|p| p.parent()).map(|p| p.join("lib64")))
        .unwrap_or_else(|| PathBuf::from("/usr/local/cuda/lib64"))
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(gpu)");
    println!("cargo:rerun-if-env-changed=CUDA_ARCH");
    println!("cargo:rerun-if-changed=cu/keccakf_witness.cu");
    println!("cargo:rerun-if-changed=cu/Makefile");

    let Some(nvcc) = find_nvcc() else {
        println!(
            "cargo:warning=[BUILD INFO] keccakf built without GPU support (CUDA not detected)"
        );
        return;
    };

    let out_dir = env::var("OUT_DIR").unwrap();
    let build_dir = Path::new(&out_dir).join("keccakf_cu");
    fs::create_dir_all(&build_dir).unwrap();

    let status = Command::new("make")
        .arg("all")
        .env("OUT_DIR", &build_dir)
        .env("NVCC", &nvcc)
        .current_dir("cu")
        .status()
        .expect("failed to run make for cu/");
    assert!(status.success(), "keccakf GPU build failed");

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=keccakf_cu");
    println!("cargo:rustc-link-search=native={}", cuda_lib_dir(&nvcc).display());
    println!("cargo:rustc-link-lib=dylib=cudart");
    println!("cargo:rustc-link-lib=dylib=stdc++");
    println!("cargo:rustc-cfg=gpu");
}
