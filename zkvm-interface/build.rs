fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let header = format!("{}/zkvm_accelerators.h", manifest_dir);

    println!("cargo:rerun-if-changed={}", header);

    let bindings = bindgen::Builder::default()
        .header(&header)
        .clang_arg("-std=c11")
        .allowlist_type("zkvm_.*")
        .allowlist_var("ZKVM_.*")
        .allowlist_function("zkvm_.*")
        .generate()
        .expect("Unable to generate bindings from zkvm_accelerators.h");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("zkvm_accelerators_bindings.rs"))
        .expect("Couldn't write bindings!");
}
