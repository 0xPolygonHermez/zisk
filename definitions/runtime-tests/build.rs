fn main() {
    println!("cargo:rerun-if-changed=native.cpp");
    println!("cargo:rerun-if-changed=../../emulator-asm/src/koala_poseidon2.hpp");
    println!("cargo:rerun-if-changed=../../emulator-asm/src/koala_poseidon2_parameters.hpp");
    cc::Build::new().cpp(true).file("native.cpp").warnings(true).compile("koala_native_test");
}
