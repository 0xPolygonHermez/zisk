fn main() {
    println!("cargo:rustc-check-cfg=cfg(zisk_guest)");
    // `src/core` is a symlink to ziskos/entrypoint/src, which references the
    // `zisk_staticlib` cfg. ziskos-hints never builds as a staticlib, so we only
    // register the name (we never set it) to keep `unexpected_cfgs` quiet.
    println!("cargo:rustc-check-cfg=cfg(zisk_staticlib)");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_vendor = std::env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if (target_os == "zkvm" && target_vendor == "zisk")
        || (target_arch == "riscv64" && target_os == "none")
    {
        println!("cargo:rustc-cfg=zisk_guest");
    }
}
