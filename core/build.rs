use std::path::{Path, PathBuf};

fn main() {
    // `lib-float` (a build-dep of this crate) publishes its built ELF at
    // `<workspace_target>/zisk-libs/ziskfloat.elf`. We resolve that path here
    // and pass it to the source via `cargo:rustc-env`, so
    // `include_bytes!(env!("ZISK_FLOAT_ELF"))` picks it up at compile time.
    //
    // We can't use cargo's `DEP_<name>_<key>` mechanism because cargo only
    // forwards `links` metadata across regular dependencies, not build-deps —
    // and `core` only needs `lib-float`'s build artifacts, not its Rust API.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let target_dir = workspace_target_dir(&out_dir);
    let elf_path = target_dir.join("zisk-libs").join("ziskfloat.elf");

    println!("cargo:rerun-if-changed={}", elf_path.display());
    println!("cargo:rustc-env=ZISK_FLOAT_ELF={}", elf_path.display());
}

/// Resolve the workspace `target/` directory from a build script.
///
/// Cargo lays `OUT_DIR` out as `<target_dir>/[<triple>/]<profile>/build/<crate>-<hash>/out`.
fn workspace_target_dir(out_dir: &Path) -> PathBuf {
    if let Ok(env_target) = std::env::var("CARGO_TARGET_DIR") {
        let p = PathBuf::from(env_target);
        if p.is_absolute() {
            return p;
        }
    }
    out_dir
        .ancestors()
        .find(|a| a.file_name().and_then(|n| n.to_str()) == Some("target"))
        .unwrap_or_else(|| panic!("No 'target' ancestor of {}", out_dir.display()))
        .to_path_buf()
}
