extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=libzkProverAPI.a");

    // Add library path to the environment for bindgen.
    // To find a library... sudo find /usr /lib /lib64 /usr/lib /usr/lib64 -name "libstdc++.a"
    println!("cargo:rustc-link-search=native={}", env::current_dir().unwrap().display());
    println!("cargo:rustc-link-search=native=/usr/lib/gcc/x86_64-linux-gnu/11/");
    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu/");

    // Link against the library using rustc-link-lib.
    println!("cargo:rustc-link-arg=-fopenmp");

    println!("cargo:rustc-link-lib=static=zkProverAPI");
    println!("cargo:rustc-link-lib=static=stdc++");
    println!("cargo:rustc-link-lib=static=gmp");
    println!("cargo:rustc-link-lib=static=gmpxx");
    println!("cargo:rustc-link-lib=static=crypto");
    println!("cargo:rustc-link-lib=static=uuid");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // Tell bindgen we are processing C++.
        .clang_arg("-xc++")
        .clang_arg("-std=c++17")
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").expect("Failed to get OUT_DIR"));
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
