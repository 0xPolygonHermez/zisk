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

/// Parse the `CUDA_ARCHS` env var:
///
/// - Unset    → `None` (delegate to Makefile autodetect)
/// - "major"  → `Some([80, 86, 89, 90, 100, 120])` — Ampere..Blackwell-consumer
/// - "89" / "89,90" → `Some(parsed_ints)`, sorted + deduped
/// - garbage  → panic with a clear message
fn parse_cuda_archs() -> Option<Vec<u32>> {
    match env::var("CUDA_ARCHS") {
        Err(_) => None,
        Ok(val) if val.trim().eq_ignore_ascii_case("major") => Some(vec![80, 86, 89, 90, 100, 120]),
        Ok(val) => {
            let mut archs = Vec::new();
            for token in val.split(',') {
                let s = token.trim();
                match s.parse::<u32>() {
                    Ok(n) => archs.push(n),
                    Err(_) => panic!(
                        "CUDA_ARCHS contains invalid entry {:?} — expected integers (e.g. '89', '89,90', or 'major')",
                        s
                    ),
                }
            }
            if archs.is_empty() {
                panic!("CUDA_ARCHS is set but empty — expected integers (e.g. '89', '89,90', or 'major')");
            }
            archs.sort_unstable();
            archs.dedup();
            Some(archs)
        }
    }
}

/// Build the `-gencode …` flag string. PTX emitted per Blackwell lineage:
/// sm_100..119 (datacenter, TMEM) is incompatible with sm_120+ (consumer,
/// no TMEM) — emit one `compute_X,code=compute_X` PTX per lineage present.
/// Single-lineage builds emit one PTX.
fn cuda_gencode_flags(archs: &[u32]) -> String {
    let mut flags = Vec::new();
    for &arch in archs {
        flags.push(format!("-gencode arch=compute_{arch},code=sm_{arch}"));
    }
    let max_dc_blackwell = archs.iter().filter(|&&a| (100..120).contains(&a)).max().copied();
    let max_other = archs.iter().filter(|&&a| !(100..120).contains(&a)).max().copied();
    match (max_dc_blackwell, max_other) {
        (Some(dc), Some(other)) => {
            flags.push(format!("-gencode arch=compute_{dc},code=compute_{dc}"));
            flags.push(format!("-gencode arch=compute_{other},code=compute_{other}"));
        }
        _ => {
            let max_arch = *archs.iter().max().expect("archs list is empty");
            flags.push(format!("-gencode arch=compute_{max_arch},code=compute_{max_arch}"));
        }
    }
    flags.join(" ")
}

fn main() {
    println!("cargo:rerun-if-env-changed=CUDA_ARCHS");
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

    // Translate CUDA_ARCHS → CUDA_GENCODE_FLAGS for the Makefile. `None`
    // means "let the Makefile autodetect via detect_cuda_arch.sh".
    let gencode_flags: Option<String> = match parse_cuda_archs() {
        None => {
            eprintln!("CUDA_ARCHS not set — Makefile will autodetect GPU arch from host");
            None
        }
        Some(archs) => {
            let flags = cuda_gencode_flags(&archs);
            eprintln!("CUDA gencode flags: {flags}");
            Some(flags)
        }
    };

    // Stamp file: force a `make clean` when the requested arch set changes
    // between invocations. Stores "auto" for autodetect or the gencode
    // string for explicit builds.
    let archs_stamp_path = gpu_build_dir.join(".cuda_archs_stamp");
    let stamp_content = gencode_flags.as_deref().unwrap_or("auto");
    let archs_changed =
        fs::read_to_string(&archs_stamp_path).map(|s| s.trim() != stamp_content).unwrap_or(true);
    if archs_changed {
        eprintln!("CUDA_ARCHS changed — running clean rebuild...");
        let _ = Command::new("make")
            .arg("clean")
            .env("OUT_DIR", &gpu_build_dir)
            .current_dir("cu")
            .status();
    }

    // Invoke the cu/Makefile. Pass NVCC explicitly so the discovered probe
    // wins over `nvcc` on PATH that may shadow it. CUDA_GENCODE_FLAGS is
    // only set when CUDA_ARCHS produced an explicit list.
    let mut make = Command::new("make");
    make.arg("all").env("OUT_DIR", &gpu_build_dir).env("NVCC", &nvcc).current_dir("cu");
    if let Some(flags) = &gencode_flags {
        make.env("CUDA_GENCODE_FLAGS", flags);
    }
    let status = make.status().expect("Failed to run make for cu/");
    assert!(status.success(), "GPU Makefile build failed");

    if let Err(e) = fs::write(&archs_stamp_path, stamp_content) {
        eprintln!(
            "Warning: failed to write CUDA arch stamp {archs_stamp_path:?}: {e} — next build will recompile"
        );
    }

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
