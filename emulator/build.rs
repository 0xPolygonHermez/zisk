use std::path::PathBuf;
use std::process::Command;

fn main() {
    emit_wasm32_wasip1_cfg();

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
}
fn emit_wasm32_wasip1_cfg() {
    println!("cargo:rustc-check-cfg=cfg(wasm32_wasip1_target)");

    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let Some(sysroot) = Command::new(rustc)
        .args(["--print", "sysroot"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
    else {
        return;
    };

    let rustlib = sysroot.join("lib/rustlib");
    let components = rustlib.join("components");
    let watched = if components.exists() { components } else { rustlib.clone() };
    println!("cargo:rerun-if-changed={}", watched.display());

    if rustlib.join("wasm32-wasip1/lib").is_dir() {
        println!("cargo:rustc-cfg=wasm32_wasip1_target");
    }
}
