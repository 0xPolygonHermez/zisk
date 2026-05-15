use std::env;
use std::path::Path;
use std::process::Command;

/// Detects whether GPU (CUDA) support is available.
/// Returns false if the `cpu-only` feature is set or if no CUDA toolkit is found.

fn detect_gpu() -> bool {
    if cfg!(feature = "cpu-only") {
        return false;
    }
    let nvcc_in_cuda = Path::new("/usr/local/cuda/bin/nvcc").exists();
    let nvcc_in_path =
        Command::new("nvcc").arg("--version").output().map(|o| o.status.success()).unwrap_or(false);
    nvcc_in_cuda || nvcc_in_path
}

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "linux" {
        return;
    }

    let out_dir = env::var("OUT_DIR").unwrap(); // Cargo sets this for each build
    let build_dir = Path::new(&out_dir).join("memcpp");

    // Ensure build path exists
    std::fs::create_dir_all(&build_dir).unwrap();

    // Build extra C++ defines based on enabled Cargo features
    let mut extra_defines = String::new();
    if cfg!(feature = "save_mem_align_counters") {
        extra_defines.push_str(" -DSAVE_MEM_ALIGN_COUNTERS");
    }
    if cfg!(feature = "save_mem_bus_data_asm") {
        extra_defines.push_str(" -DSAVE_MEM_BUS_DATA_ASM");
    }

    // Call make with an output path override
    let status = Command::new("make")
        .arg("all")
        .env("OUT_DIR", &build_dir)
        .env("EXTRA_CXXFLAGS", &extra_defines)
        .current_dir("cpp")
        .status()
        .expect("Failed to run make");

    assert!(status.success(), "Makefile build failed");

    // Tell Cargo where to find the library
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=memcpp");
    println!("cargo:rustc-link-lib=dylib=stdc++");
    // libmemcpp.a is compiled with -fopenmp; the consumer must link gomp.
    println!("cargo:rustc-link-lib=dylib=gomp");

    watch_dir_recursive("cpp", &["cpp", "hpp"]);

    // GPU library — auto-detected
    let use_gpu = if cfg!(feature = "cpu-only") {
        println!("cargo:warning=[BUILD INFO] mem-planner-cpp compiled with CPU-only support (feature enabled)");
        false
    } else if detect_gpu() {
        println!("cargo:warning=[BUILD INFO] mem-planner-cpp compiled with GPU support");
        true
    } else {
        println!("cargo:warning=[BUILD INFO] mem-planner-cpp compiled with CPU-only support (CUDA not detected)");
        false
    };

    if use_gpu {
        let gpu_build_dir = Path::new(&out_dir).join("memcpp_cu");
        std::fs::create_dir_all(&gpu_build_dir).unwrap();

        let path = format!("/usr/local/cuda/bin:{}", env::var("PATH").unwrap_or_default());
        let status = Command::new("make")
            .arg("all")
            .env("OUT_DIR", &gpu_build_dir)
            .env("PATH", &path)
            .current_dir("cu")
            .status()
            .expect("Failed to run make");

        assert!(status.success(), "GPU Makefile build failed");

        println!("cargo:rustc-link-search=native={}", gpu_build_dir.display());
        println!("cargo:rustc-link-lib=static=memcpp_cu");
        println!("cargo:rustc-link-search=native=/usr/local/cuda/lib64");
        println!("cargo:rustc-link-lib=dylib=cudart");

        watch_dir_recursive("cu", &["cu", "cuh"]);
        println!("cargo:rustc-cfg=gpu");
    }
    println!("cargo:rustc-check-cfg=cfg(gpu)");
}

fn watch_dir_recursive<P: AsRef<Path>>(dir: P, exts: &[&str]) {
    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            watch_dir_recursive(&path, exts);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if exts.iter().any(|e| *e == ext) {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}
