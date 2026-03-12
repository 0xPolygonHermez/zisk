use std::path::Path;
use std::process::Command;

fn main() {
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-cfg=feature=\"no_lib_link\"");
        return;
    }

    // Paths
    let c_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("c");
    if !c_path.exists() {
        panic!("Missing c_path = {}", c_path.display());
    }
    let library_folder = c_path.join("lib");
    let build_folder = c_path.join("build");
    let library_name = "ziskc";
    let lib_file = library_folder.join(format!("lib{library_name}.a"));

    // Ensure build and lib directories exist before running make
    std::fs::create_dir_all(&build_folder)
        .unwrap_or_else(|e| panic!("Failed to create build directory: {e}"));
    std::fs::create_dir_all(&library_folder)
        .unwrap_or_else(|e| panic!("Failed to create lib directory: {e}"));

    // Run make (incremental build - only recompiles changed files)
    let status = Command::new("make")
        .current_dir(&c_path)
        .status()
        .unwrap_or_else(|e| panic!("Failed to execute `make`: {e}"));

    if !status.success() {
        panic!("Command `make` failed with exit code {:?}", status.code());
    }

    // Verify the library exists after build
    if !lib_file.exists() {
        panic!("`{}` was not found after build", lib_file.display());
    }

    // Absolute path to the library
    let abs_lib_path = library_folder.canonicalize().unwrap_or_else(|_| library_folder.clone());

    // Link the static library
    println!("cargo:rustc-link-search=native={}", abs_lib_path.display());
    println!("cargo:rustc-link-lib=static={library_name}");

    // Track C source files for recompilation
    track_sources(&c_path);

    // Link required libraries
    for lib in &["pthread", "gmp", "stdc++", "gmpxx", "c"] {
        println!("cargo:rustc-link-lib={lib}");
    }
}

/// Tell Cargo to track C source files for changes
fn track_sources(dir: &Path) {
    // Track all C/C++ source files and headers recursively
    if let Ok(entries) = std::fs::read_dir(dir.join("src")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                track_sources_recursive(&path);
            } else if let Some(ext) = path.extension() {
                if ext == "c" || ext == "cpp" || ext == "h" || ext == "hpp" || ext == "asm" {
                    println!("cargo:rerun-if-changed={}", path.display());
                }
            }
        }
    }

    // Also track the Makefile itself
    println!("cargo:rerun-if-changed={}", dir.join("Makefile").display());
}

fn track_sources_recursive(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                track_sources_recursive(&path);
            } else if let Some(ext) = path.extension() {
                if ext == "c" || ext == "cpp" || ext == "h" || ext == "hpp" || ext == "asm" {
                    println!("cargo:rerun-if-changed={}", path.display());
                }
            }
        }
    }
}
