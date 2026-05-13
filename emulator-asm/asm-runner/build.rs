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
    if detect_gpu() {
        println!("cargo:rustc-cfg=gpu");
    }
    println!("cargo:rustc-check-cfg=cfg(gpu)");
}
