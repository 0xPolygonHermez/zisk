use std::path::Path;
use std::process::Command;

/// Detect CUDA presence using the same probe cascade as `mem-cpp/build.rs`:
/// nvcc on PATH, `/usr/local/cuda/bin/nvcc`, `/opt/cuda/bin/nvcc`. Returns
/// false on macOS or under `cpu-only`.
fn detect_gpu() -> bool {
    if cfg!(feature = "cpu-only") {
        return false;
    }
    if cfg!(target_os = "macos") {
        return false;
    }
    if Command::new("nvcc").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
    {
        return true;
    }
    for candidate in ["/usr/local/cuda/bin/nvcc", "/opt/cuda/bin/nvcc"] {
        if Path::new(candidate).exists() {
            return true;
        }
    }
    false
}

fn main() {
    if detect_gpu() {
        println!("cargo:rustc-cfg=gpu");
    }
    println!("cargo::rustc-check-cfg=cfg(gpu)");
}
