use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Locate an `nvcc` binary. Probe order:
/// 1. `nvcc` on `PATH`
/// 2. `/usr/local/cuda/bin/nvcc`
/// 3. `/opt/cuda/bin/nvcc`
///
/// Returns `None` on macOS (no CUDA), under the `cpu-only` feature, or
/// when no candidate is found.
fn find_nvcc() -> Option<PathBuf> {
    if cfg!(feature = "cpu-only") {
        return None;
    }
    if cfg!(target_os = "macos") {
        return None;
    }
    if Command::new("nvcc").arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
        return Some(PathBuf::from("nvcc"));
    }
    for candidate in ["/usr/local/cuda/bin/nvcc", "/opt/cuda/bin/nvcc"] {
        if Path::new(candidate).exists() {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

/// Derive the CUDA library directory from a discovered `nvcc` path
/// (`<prefix>/bin/nvcc` → `<prefix>/lib64`). Falls back to
/// `/usr/local/cuda/lib64` when nvcc was found via `PATH` only.
fn cuda_lib_dir(nvcc: &Path) -> PathBuf {
    if let Ok(abs) = nvcc.canonicalize() {
        if let Some(parent) = abs.parent().and_then(|p| p.parent()) {
            return parent.join("lib64");
        }
    }
    PathBuf::from("/usr/local/cuda/lib64")
}

fn main() {
    println!("cargo:rerun-if-env-changed=CUDA_ARCHS");
    println!("cargo:rerun-if-env-changed=CUDA_ARCH");
    println!("cargo:rerun-if-env-changed=CUDA_GENCODE_FLAGS");
    println!("cargo::rustc-check-cfg=cfg(gpu)");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let is_macos = target_os == "macos";

    let out_dir = env::var("OUT_DIR").unwrap();
    let build_dir = Path::new(&out_dir).join("memcpp");
    fs::create_dir_all(&build_dir).unwrap();

    // Build extra C++ defines based on enabled Cargo features
    let mut extra_defines = String::new();
    if cfg!(feature = "save_mem_align_counters") {
        extra_defines.push_str(" -DSAVE_MEM_ALIGN_COUNTERS");
    }
    if cfg!(feature = "save_mem_bus_data_asm") {
        extra_defines.push_str(" -DSAVE_MEM_BUS_DATA_ASM");
    }

    // Build CPU library
    let status = Command::new("make")
        .arg("all")
        .env("OUT_DIR", &build_dir)
        .env("EXTRA_CXXFLAGS", &extra_defines)
        .current_dir("cpp")
        .status()
        .expect("Failed to run make");
    assert!(status.success(), "Makefile build failed");

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=memcpp");

    // OpenMP runtime + C++ stdlib differ by platform.
    if is_macos {
        // libomp from Homebrew (clang's -fopenmp resolves via libomp).
        let brew_prefix = Command::new("brew")
            .arg("--prefix")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "/opt/homebrew".to_string());
        println!("cargo:rustc-link-search=native={brew_prefix}/lib");
        println!("cargo:rustc-link-search=native={brew_prefix}/opt/libomp/lib");
        println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=dylib=omp");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        // libmemcpp.a is compiled with -fopenmp; the consumer must link gomp.
        println!("cargo:rustc-link-lib=dylib=gomp");
    }

    watch_dir_recursive("cpp", &["cpp", "hpp"]);

    // GPU library — Linux only, requires nvcc.
    let nvcc = find_nvcc();
    let use_gpu = match (&nvcc, cfg!(feature = "cpu-only"), is_macos) {
        (_, true, _) => {
            println!("cargo:warning=[BUILD INFO] mem-planner-cpp compiled with CPU-only support (feature enabled)");
            false
        }
        (_, _, true) => {
            println!("cargo:warning=[BUILD INFO] mem-planner-cpp compiled with CPU-only support (macOS — no CUDA)");
            false
        }
        (None, _, _) => {
            println!("cargo:warning=[BUILD INFO] mem-planner-cpp compiled with CPU-only support (CUDA not detected)");
            false
        }
        (Some(_), _, _) => {
            println!("cargo:warning=[BUILD INFO] mem-planner-cpp compiled with GPU support");
            true
        }
    };

    if !use_gpu {
        return;
    }
    let nvcc = nvcc.unwrap();

    let gpu_build_dir = Path::new(&out_dir).join("memcpp_cu");
    fs::create_dir_all(&gpu_build_dir).unwrap();

    // Invoke the cu/Makefile, which owns all CUDA arch resolution (CUDA_ARCHS
    // et al. reach it via the process environment).
    let status = Command::new("make")
        .arg("all")
        .env("OUT_DIR", &gpu_build_dir)
        .env("NVCC", &nvcc)
        .current_dir("cu")
        .status()
        .expect("Failed to run make for cu/");
    assert!(status.success(), "GPU Makefile build failed");

    println!("cargo:rustc-link-search=native={}", gpu_build_dir.display());
    println!("cargo:rustc-link-lib=static=memcpp_cu");

    let cuda_lib = cuda_lib_dir(&nvcc);
    println!("cargo:rustc-link-search=native={}", cuda_lib.display());
    println!("cargo:rustc-link-lib=dylib=cudart");

    watch_dir_recursive("cu", &["cu", "cuh"]);
    println!("cargo:rerun-if-changed=cu/Makefile");
    println!("cargo:rerun-if-changed=cu/detect_cuda_arch.sh");
    println!("cargo:rustc-cfg=gpu");
}

fn watch_dir_recursive<P: AsRef<Path>>(dir: P, exts: &[&str]) {
    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            watch_dir_recursive(&path, exts);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if exts.contains(&ext) {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}
