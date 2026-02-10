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

    let disable_distributed =
        std::env::vars().any(|(k, _)| k == "CARGO_FEATURE_DISABLE_DISTRIBUTED");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    // Distributed feature is only available on linux x86_64
    if !disable_distributed && target_os == "linux" && target_arch == "x86_64" {
        println!("cargo:rustc-cfg=distributed");
    }
}
