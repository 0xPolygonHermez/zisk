fn main() {
    println!("cargo:rustc-check-cfg=cfg(zisk_guest)");
    println!("cargo:rustc-check-cfg=cfg(zisk_staticlib)");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_vendor = std::env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if (target_os == "zkvm" && target_vendor == "zisk")
        || (target_arch == "riscv64" && target_os == "none")
    {
        println!("cargo:rustc-cfg=zisk_guest");
    }

    // `ziskos` is consumed in two ways:
    //   1. directly by path from a guest program (ziskos *is* the whole no_std
    //      binary and must own the `#[global_allocator]`), and
    //   2. through `ziskos-staticlib`, which produces `libziskos.a` to be linked
    //      into a host application (Rust or C). In that case ziskos must NOT
    //      register a `#[global_allocator]`, since it would collide with (or
    //      hijack) the host application's allocator.
    //
    // A dependency cannot autodetect the crate-type of who links it, so the
    // staticlib build opts in via the `staticlib` feature. We surface it as the
    // `zisk_staticlib` cfg to keep call sites consistent with `zisk_guest`.
    if std::env::var_os("CARGO_FEATURE_STATICLIB").is_some() {
        println!("cargo:rustc-cfg=zisk_staticlib");
    }
}
