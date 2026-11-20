fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let accelerators_header = format!("{}/zkvm_accelerators.h", manifest_dir);
    let accelerators_ext_header = format!("{}/zkvm_accelerators_ext.h", manifest_dir);
    let io_header = format!("{}/zkvm_io.h", manifest_dir);

    println!("cargo:rerun-if-changed={}", accelerators_header);
    println!("cargo:rerun-if-changed={}", accelerators_ext_header);
    println!("cargo:rerun-if-changed={}", io_header);

    // A target without an OS, such as a guest's `riscv64ima-zisk-zkvm-elf`, is freestanding.
    // Saying so matters when bindgen's include path detection runs a `clang` of another version
    // than the libclang it loads, as on a host with several LLVM versions: both versions'
    // builtin headers are then on the search path, and a hosted `stdint.h` defers to the next
    // one with `#include_next`, which shares its include guard and so defines nothing. A
    // freestanding `stdint.h` defines the types itself.
    let bare_metal =
        matches!(std::env::var("CARGO_CFG_TARGET_OS").as_deref(), Ok("none") | Ok("zkvm"));

    let bindings = bindgen::Builder::default()
        .header(&accelerators_header)
        .header(&accelerators_ext_header)
        .header(&io_header)
        .clang_arg("-std=c11")
        .clang_args(bare_metal.then_some("-ffreestanding"))
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
