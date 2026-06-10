use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "linux" {
        return;
    }

    // Paths
    let c_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("c");
    if !c_path.exists() {
        panic!("Missing c_path = {}", c_path.display());
    }

    // Build artifacts go under Cargo's OUT_DIR so `cargo clean` removes them
    // (previously they lived in c/build and c/lib, in-source, and survived clean).
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set by Cargo"));
    let build_folder = out_dir.join("build");
    let library_folder = out_dir.join("lib");
    let library_name = "ziskc";
    let lib_file = library_folder.join(format!("lib{library_name}.a"));

    // Ensure build and lib directories exist before running make
    std::fs::create_dir_all(&build_folder)
        .unwrap_or_else(|e| panic!("Failed to create build directory: {e}"));
    std::fs::create_dir_all(&library_folder)
        .unwrap_or_else(|e| panic!("Failed to create lib directory: {e}"));

    // Run make (incremental build - only recompiles changed files), pointing its
    // BUILD_DIR / LIB_DIR at OUT_DIR so nothing is written into the source tree.
    let status = Command::new("make")
        .arg(format!("BUILD_DIR={}", build_folder.display()))
        .arg(format!("LIB_DIR={}", library_folder.display()))
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

    // Publish a copy under <target>/zisk-libs/ for tools that link libziskc
    // outside cargo (emulator-asm `-L`, rom-setup, build_zisk.sh). Those tools
    // all build lib-c natively from the workspace, where OUT_DIR is
    // <workspace>/target/<profile>/build/lib-c-<hash>/out, so locating the
    // `target` ancestor lands the copy at <workspace>/target/zisk-libs/ as they
    // expect. This is a best-effort side-channel: if OUT_DIR has no ancestor
    // named `target` (e.g. CARGO_TARGET_DIR set to a differently-named dir),
    // skip rather than fail — every build that lacks one is also one that never
    // reads zisk-libs (the guest/ELF build links the installed lib instead).
    if let Some(target_dir) =
        out_dir.ancestors().find(|a| a.file_name().and_then(|n| n.to_str()) == Some("target"))
    {
        let runtime_dir = target_dir.join("zisk-libs");
        std::fs::create_dir_all(&runtime_dir)
            .unwrap_or_else(|e| panic!("Failed to create runtime lib dir: {e}"));
        let runtime_lib = runtime_dir.join(format!("lib{library_name}.a"));
        std::fs::copy(&lib_file, &runtime_lib).unwrap_or_else(|e| {
            panic!("Failed to copy {} to {}: {e}", lib_file.display(), runtime_lib.display())
        });
    }

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
