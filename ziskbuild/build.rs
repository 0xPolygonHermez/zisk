fn main() {
    let mut builder = vergen_git2::Emitter::default();
    builder
        .add_instructions(
            &vergen_git2::BuildBuilder::default().build_timestamp(true).build().unwrap(),
        )
        .unwrap();
    builder
        .add_instructions(&vergen_git2::Git2Builder::default().sha(true).build().unwrap())
        .unwrap();
    builder.emit().unwrap();

    // Determine compute mode for version string.
    // If the cpu-only feature is enabled, always report "cpu".
    // Otherwise, auto-detect CUDA: check common install paths, CUDA_HOME env var,
    // or whether `nvcc` is available on PATH.
    let cpu_only = std::env::vars().any(|(k, _)| k == "CARGO_FEATURE_CPU_ONLY");
    let compute_mode = if cpu_only {
        "cpu"
    } else {
        // Check for nvcc itself, not just the directory: a stale /usr/local/cuda
        // (e.g. old samples on macOS) must not flip the label to "gpu" when
        // proofman-starks-lib-c's detection (which requires nvcc) builds CPU.
        let has_cuda = std::path::Path::new("/usr/local/cuda/bin/nvcc").exists()
            || std::env::var("CUDA_HOME").is_ok()
            || std::process::Command::new("nvcc")
                .arg("--version")
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        if has_cuda {
            "gpu"
        } else {
            "cpu"
        }
    };
    println!("cargo:rustc-env=ZISK_COMPUTE_MODE={compute_mode}");
}
