fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let accelerators_header = format!("{}/zkvm_accelerators.h", manifest_dir);
    let io_header = format!("{}/zkvm_io.h", manifest_dir);

    println!("cargo:rerun-if-changed={}", accelerators_header);
    println!("cargo:rerun-if-changed={}", io_header);

    let bindings = bindgen::Builder::default()
        .header(&accelerators_header)
        .header(&io_header)
        .clang_arg("-std=c11")
        .use_core()
        .allowlist_type("zkvm_.*")
        .allowlist_var("ZKVM_.*")
        .allowlist_function("^zkvm_.*|^read_input$|^write_output$")
        .generate()
        .expect("Unable to generate bindings from zkvm-interface headers");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("zkvm_interface_bindings.rs"))
        .expect("Couldn't write bindings!");
}
