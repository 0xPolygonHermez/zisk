//! Build-script support for zisk crates that ship CUDA kernels.
//!
//! Each such crate used to carry its own `build.rs` nvcc probe, its own
//! `cu/Makefile` and its own copy of `detect_cuda_arch.sh`. That is one copy
//! per crate of the arch-resolution logic, and — more dangerously — one
//! chance per crate to get the CUDA runtime link mode wrong. A kernel that
//! pulls in `libcudart.so` while proofman links `cudart_static` puts two
//! independent CUDA runtimes in one process, each with its own thread-local
//! current device: a kernel launch then lands on device 0 whatever device the
//! caller selected, which is invisible on one GPU and silently wrong on
//! several.
//!
//! So this crate owns the Makefile, the detection script and the link
//! directives, and a consumer's `build.rs` is one call:
//!
//! ```ignore
//! fn main() {
//!     zisk_cuda_build::compile(&zisk_cuda_build::CudaLib {
//!         name: "keccakf_cu",
//!         dir: "cu",
//!         sources: &["keccakf_witness.cu"],
//!         ..Default::default()
//!     });
//! }
//! ```
//!
//! `cfg(gpu)` is emitted when — and only when — the archive was built, so the
//! consuming crate gates its GPU code on it and still builds on a host with
//! no nvcc.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A CUDA static library to build from one crate's sources.
pub struct CudaLib<'a> {
    /// Archive name without the `lib` prefix or `.a` suffix, e.g. `"keccakf_cu"`.
    /// Must be unique across the workspace: every archive ends up in one link.
    pub name: &'a str,
    /// Directory holding the `.cu` sources, relative to the consuming crate's root.
    pub dir: &'a str,
    /// Source file names within [`dir`](Self::dir).
    pub sources: &'a [&'a str],
    /// Extra include directories, relative to the crate root. Their headers are
    /// added to `-I` and watched for changes.
    pub extra_header_dirs: &'a [&'a str],
    /// Appended to `NVCCFLAGS`, e.g. `-DSOMETHING`. Keep empty unless needed:
    /// flags that change code generation belong in the source, not per crate.
    pub extra_nvcc_flags: &'a str,
}

impl Default for CudaLib<'_> {
    fn default() -> Self {
        Self { name: "", dir: "cu", sources: &[], extra_header_dirs: &[], extra_nvcc_flags: "" }
    }
}

/// Locate an `nvcc` binary as an ABSOLUTE path.
///
/// Absolute matters: [`cuda_lib_dir`] derives `lib64` from it, and a bare
/// `"nvcc"` does not canonicalize (`Path::canonicalize` does no `PATH`
/// lookup), so it would silently fall back to `/usr/local/cuda/lib64` — wrong
/// on an `/opt/cuda` host.
///
/// Returns `None` on macOS, under the consuming crate's `cpu-only` feature, or
/// when no candidate runs.
pub fn find_nvcc() -> Option<PathBuf> {
    if skip_reason().is_some() {
        return None;
    }
    let from_path = env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).map(|dir| dir.join("nvcc")).collect::<Vec<_>>());
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

/// Why the GPU build is being skipped before nvcc is even looked for, for the
/// build warning. `None` means go ahead and probe.
fn skip_reason() -> Option<&'static str> {
    // The consuming crate's features, not this crate's: `cfg!(feature = ..)`
    // here would read zisk-cuda-build's own (empty) feature set.
    if env::var_os("CARGO_FEATURE_CPU_ONLY").is_some() {
        return Some("the cpu-only feature is enabled");
    }
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        return Some("macOS has no CUDA");
    }
    None
}

/// `<prefix>/bin/nvcc` -> `<prefix>/lib64`, following symlinks (the usual
/// `/usr/local/cuda` -> `/usr/local/cuda-12.9` indirection).
pub fn cuda_lib_dir(nvcc: &Path) -> PathBuf {
    nvcc.canonicalize()
        .ok()
        .and_then(|abs| abs.parent().and_then(|p| p.parent()).map(|p| p.join("lib64")))
        .unwrap_or_else(|| PathBuf::from("/usr/local/cuda/lib64"))
}

/// Build `lib` and emit the cargo directives to link it.
///
/// Returns whether the archive was built. When it was, `cfg(gpu)` has been
/// emitted for the consuming crate; when it wasn't (no nvcc, macOS, or the
/// `cpu-only` feature) nothing is linked and the crate must compile with its
/// GPU code gated out.
///
/// Panics if nvcc is present but the build fails: a half-built GPU crate that
/// links is worse than a loud failure.
pub fn compile(lib: &CudaLib<'_>) -> bool {
    assert!(!lib.name.is_empty(), "CudaLib::name is required");
    assert!(!lib.sources.is_empty(), "CudaLib::sources is required");

    // Always, so the consuming crate never warns about an unknown cfg on a
    // host where the GPU build is skipped.
    println!("cargo::rustc-check-cfg=cfg(gpu)");
    for var in ["CUDA_ARCH", "CUDA_ARCHS", "CUDA_GENCODE_FLAGS"] {
        println!("cargo:rerun-if-env-changed={var}");
    }

    let crate_name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "crate".into());
    let Some(nvcc) = find_nvcc() else {
        let why = skip_reason().unwrap_or("nvcc not found");
        println!("cargo:warning=[BUILD INFO] {crate_name} built without GPU support ({why})");
        return false;
    };

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let src_dir = manifest_dir.join(lib.dir);
    assert!(src_dir.is_dir(), "{crate_name}: CUDA source dir {} does not exist", src_dir.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join(lib.name);
    std::fs::create_dir_all(&out_dir).expect("create OUT_DIR");

    // This crate is a workspace path dependency and never published, so its
    // source tree is on disk when a consumer's build script runs.
    let make_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("make");
    let shared_include = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("include");
    let extra_includes: Vec<String> =
        lib.extra_header_dirs.iter().map(|d| manifest_dir.join(d).display().to_string()).collect();

    let status = Command::new("make")
        .arg("-f")
        .arg(make_dir.join("Makefile"))
        .arg("all")
        .env("NVCC", &nvcc)
        .env("OUT_DIR", &out_dir)
        .env("SRC_DIR", &src_dir)
        .env("SRCS", lib.sources.join(" "))
        .env("TARGET", format!("lib{}.a", lib.name))
        .env("DETECT_SCRIPT", make_dir.join("detect_cuda_arch.sh"))
        .env("SHARED_INCLUDE", &shared_include)
        .env("EXTRA_HEADER_DIRS", extra_includes.join(" "))
        .env("EXTRA_NVCCFLAGS", lib.extra_nvcc_flags)
        .status()
        .unwrap_or_else(|e| panic!("{crate_name}: failed to run make for {}: {e}", lib.dir));
    assert!(status.success(), "{crate_name}: CUDA build failed for {}", lib.name);

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static={}", lib.name);
    // The search path only; deliberately NO `-lcudart` here. The static
    // runtime arrives once, from proofman-starks-lib-c's `cudart_static`,
    // and every kernel in the process must share it — see the module docs.
    println!("cargo:rustc-link-search=native={}", cuda_lib_dir(&nvcc).display());

    for source in lib.sources {
        println!("cargo:rerun-if-changed={}", src_dir.join(source).display());
    }
    watch_headers(&src_dir);
    for dir in lib.extra_header_dirs {
        watch_headers(&manifest_dir.join(dir));
    }
    println!("cargo:rerun-if-changed={}", make_dir.join("Makefile").display());
    println!("cargo:rerun-if-changed={}", make_dir.join("detect_cuda_arch.sh").display());
    watch_headers(&shared_include);

    println!("cargo:warning=[BUILD INFO] {crate_name} compiled with GPU support");
    println!("cargo:rustc-cfg=gpu");
    true
}

/// Re-run the build when any CUDA or C++ header in `dir` changes.
fn watch_headers(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            watch_headers(&path);
        } else if matches!(path.extension().and_then(|e| e.to_str()), Some("cuh" | "hpp" | "h")) {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
