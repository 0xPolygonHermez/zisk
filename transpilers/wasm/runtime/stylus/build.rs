fn main() {
    println!("cargo:rerun-if-changed=bridge.wat");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let wasm = wat::parse_file("bridge.wat").expect("valid Stylus bridge");
    std::fs::write(output.join("bridge.wasm"), wasm).unwrap();
}
