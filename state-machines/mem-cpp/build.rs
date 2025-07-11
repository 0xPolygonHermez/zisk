use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-cfg=feature=\"no_lib_link\"");
        return;
    }

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

    watch_dir_recursive("cpp");
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
