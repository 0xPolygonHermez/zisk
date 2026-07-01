fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let guest_elf = std::path::Path::new(&manifest_dir).join("examples/guest.elf");
    println!("cargo:rustc-env=ZISK_ELF_guest={}", guest_elf.display());
    println!("cargo:rerun-if-changed=examples/guest.elf");
}
