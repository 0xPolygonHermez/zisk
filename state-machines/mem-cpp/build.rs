use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // The CUDA env vars, `cfg(gpu)`'s check-cfg and the GPU rebuild triggers are
    // all emitted by `zisk_cuda_build::compile` at the end of this function.
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

    // GPU library — Linux only, requires nvcc. Arch resolution, the link mode
    // and cfg(gpu) all live in zisk-cuda-build so every kernel-bearing crate
    // shares one copy; see that crate's docs for why the runtime link mode in
    // particular must not be decided per crate.
    zisk_cuda_build::compile(&zisk_cuda_build::CudaLib {
        name: "memcpp_cu",
        dir: "cu",
        sources: &["count_and_plan.cu", "count_and_plan_c.cu"],
        // The kernels include the CPU side's headers.
        extra_header_dirs: &["cpp"],
        ..Default::default()
    });
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
