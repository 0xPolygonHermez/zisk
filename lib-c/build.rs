use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "linux" {
        return;
    }

    // Source path
    let c_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("c");
    if !c_path.exists() {
        panic!("Missing c_path = {}", c_path.display());
    }

    // Build/output directories under OUT_DIR so `cargo clean` removes them.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set by cargo"));
    let build_folder = out_dir.join("build");
    let library_folder = out_dir.join("lib");
    let library_name = "ziskc";
    let lib_file = library_folder.join(format!("lib{library_name}.a"));

    std::fs::create_dir_all(&build_folder)
        .unwrap_or_else(|e| panic!("Failed to create build directory: {e}"));
    std::fs::create_dir_all(&library_folder)
        .unwrap_or_else(|e| panic!("Failed to create lib directory: {e}"));

    // Run make with overridden BUILD_DIR/LIB_DIR so all artifacts land in OUT_DIR.
    let status = Command::new("make")
        .arg(format!("BUILD_DIR={}", build_folder.display()))
        .arg(format!("LIB_DIR={}", library_folder.display()))
        .current_dir(&c_path)
        .status()
        .unwrap_or_else(|e| panic!("Failed to execute `make`: {e}"));

    if !status.success() {
        panic!("Command `make` failed with exit code {:?}", status.code());
    }

    if !lib_file.exists() {
        panic!("`{}` was not found after build", lib_file.display());
    }

    // Publish the static library at a stable workspace-relative location so
    // runtime callers (e.g. emulator-asm/Makefile invoked by rom-setup) can
    // find it without knowing OUT_DIR. Living under target/ means `cargo
    // clean` removes it too.
    let runtime_dir = workspace_target_dir(&out_dir).join("zisk-libs");
    std::fs::create_dir_all(&runtime_dir)
        .unwrap_or_else(|e| panic!("Failed to create runtime lib dir: {e}"));
    let runtime_lib = runtime_dir.join(format!("lib{library_name}.a"));
    std::fs::copy(&lib_file, &runtime_lib).unwrap_or_else(|e| {
        panic!("Failed to copy {} to {}: {e}", lib_file.display(), runtime_lib.display())
    });

    // Expose the lib directory to dependent crates and to the workspace
    // (`DEP_ZISKC_LIB_DIR` for crates with a build script, `cargo:rustc-env`
    // for the `lib-c` crate itself).
    println!("cargo:lib_dir={}", library_folder.display());
    println!("cargo:runtime_lib_dir={}", runtime_dir.display());

    // Link the static library
    println!("cargo:rustc-link-search=native={}", library_folder.display());
    println!("cargo:rustc-link-lib=static={library_name}");

    // Track C source files for recompilation
    track_sources(&c_path);

    // Link required libraries
    for lib in &["pthread", "gmp", "stdc++", "gmpxx", "c"] {
        println!("cargo:rustc-link-lib={lib}");
    }
}

/// Resolve the workspace `target/` directory from a build script.
///
/// Cargo lays `OUT_DIR` out as `<target_dir>/[<triple>/]<profile>/build/<crate>-<hash>/out`.
fn workspace_target_dir(out_dir: &Path) -> PathBuf {
    if let Ok(env_target) = std::env::var("CARGO_TARGET_DIR") {
        let p = PathBuf::from(env_target);
        if p.is_absolute() {
            return p;
        }
    }
    out_dir
        .ancestors()
        .find(|a| a.file_name().and_then(|n| n.to_str()) == Some("target"))
        .unwrap_or_else(|| panic!("No 'target' ancestor of {}", out_dir.display()))
        .to_path_buf()
}

/// Tell Cargo to track C source files for changes
fn track_sources(dir: &Path) {
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
