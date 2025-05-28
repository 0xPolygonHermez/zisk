use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap(); // Cargo sets this for each build
    let build_dir = Path::new(&out_dir).join("memcpp");

    // Ensure build path exists
    std::fs::create_dir_all(&build_dir).unwrap();

    // Call make with an output path override
    let status = Command::new("make")
        .arg("all")
        .env("OUT_DIR", &build_dir)
        .current_dir("cpp")
        .status()
        .expect("Failed to run make");

    assert!(status.success(), "Makefile build failed");

    // Tell Cargo where to find the library
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=memcpp");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}
