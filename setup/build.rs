// Exposes Cargo.toml's `setup_version` line (the one CI reads too) as ZISK_SETUP_VERSION.
fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    let manifest = std::fs::read_to_string("Cargo.toml").unwrap();
    let version = manifest
        .lines()
        .find_map(|l| {
            let v = l.strip_prefix("setup_version")?.trim_start().strip_prefix('=')?.trim();
            v.strip_prefix('"')?.strip_suffix('"')
        })
        .expect("setup/Cargo.toml has no setup_version");
    println!("cargo:rustc-env=ZISK_SETUP_VERSION={version}");
}
