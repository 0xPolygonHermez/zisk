use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    watch_dir_recursive("cpp");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let build_dir = Path::new(&out_dir).join("memcpp");

    // Create build directory
    std::fs::create_dir_all(&build_dir)
        .expect(&format!("Failed to create {}", build_dir.display()));

    // Call make with an output path override
    let status = Command::new("make")
        .arg("all")
        .env("OUT_DIR", &build_dir)
        .current_dir("cpp")
        .status()
        .expect("Failed to run make");

    assert!(status.success(), "Makefile build failed");

    // Verify library was created
    let lib_path = build_dir.join("libmemcpp.a");
    if !lib_path.exists() {
        panic!("Expected library not found at {}", lib_path.display());
    }

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=memcpp");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}

fn watch_dir_recursive<P: AsRef<Path>>(dir: P) {
    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            watch_dir_recursive(&path);
        } else if let Some(ext) = path.extension() {
            if ext == "cpp" || ext == "hpp" {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}