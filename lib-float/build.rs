use std::fs;
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
    let library_folder = c_path.join("lib");
    let library_name = "ziskfloat";
    let lib_file = library_folder.join(format!("lib{library_name}.a"));
    let elf_file = library_folder.join(format!("{library_name}.elf"));

    // The committed lib/ziskfloat.elf is the program vk source of truth (see
    // core/src/elf2rom.rs include_bytes!), so we only rebuild it in workspace dev — never from
    // cargo caches or CI, where the local riscv toolchain would risk producing different bytes.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let is_consumer = manifest_dir.contains("/.cargo/git/checkouts/")
        || manifest_dir.contains("/.cargo/registry/src/");
    let in_ci = std::env::var("CI").map(|v| v == "true" || v == "1").unwrap_or(false);

    if is_consumer || in_ci {
        if !lib_file.exists() || !elf_file.exists() {
            println!(
                "cargo:warning=ziskfloat artifacts missing in cargo cache; rebuilding from source"
            );
            run_command("make", &["clean"], &c_path);
            run_command("make", &[], &c_path);
        } else {
            println!("ziskfloat artifacts already present, skipping rebuild.");
        }
    } else {
        // Workspace dev: only rebuild when a C source has actually changed since
        // libziskfloat.a was last produced. Mtimes are reliable here (unlike cargo
        // git/registry checkouts), so this skips unnecessary C rebuilds on `cargo build`.
        let cpp_files = find_cpp_files(&c_path);
        if cpp_files_have_changed(&cpp_files, &lib_file) {
            eprintln!("Changes detected! Running `make clean` and recompiling...");
            run_command("make", &["clean"], &c_path);
            run_command("make", &[], &c_path);
        } else {
            println!("No C++ source changes detected, skipping rebuild.");
        }
    }

    // Absolute path to the library
    let abs_lib_path = library_folder.canonicalize().unwrap_or_else(|_| library_folder.clone());

    if !lib_file.exists() {
        panic!("`{}` was not found", lib_file.display());
    }

    // Ensure Rust triggers a rebuild if the C++ source code changes
    track_cpp_changes(&c_path);

    // Link the static library
    println!("cargo:rustc-link-search=native={}", abs_lib_path.display());
    println!("cargo:rustc-link-lib=static={library_name}");

    // Link required libraries
    for lib in &["pthread", "gmp", "stdc++", "gmpxx", "c"] {
        println!("cargo:rustc-link-lib={lib}");
    }
}

// /// Runs an external command and checks for errors
fn run_command(cmd: &str, args: &[&str], dir: &Path) {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(dir)
        // Neutralize timestamps gcc/ar/ld might otherwise embed.
        .env("SOURCE_DATE_EPOCH", "0")
        .status()
        .unwrap_or_else(|e| panic!("Failed to execute `{cmd}`: {e}"));

    if !status.success() {
        panic!("Command `{}` failed with exit code {:?}", cmd, status.code());
    }
}

/// Tracks changes in the `pil2-stark` directory to trigger recompilation only when needed
fn track_cpp_changes(c_path: &Path) {
    println!("cargo:rerun-if-changed={}", c_path.join("Makefile").display());
    let cpp_files = find_cpp_files(c_path);
    // Print tracked files for debugging
    eprintln!("Tracking {} C++ source files:", cpp_files.len());
    for file in &cpp_files {
        eprintln!(" - {}", file.display());
        println!("cargo:rerun-if-changed={}", file.display());
    }
}

/// Checks if any source file has been modified after `libziskfloat.a` was last built
fn cpp_files_have_changed(cpp_files: &[PathBuf], lib_file: &Path) -> bool {
    let mut modified_files: Vec<PathBuf> = Vec::new();
    let lib_modified_time = match fs::metadata(lib_file) {
        Ok(metadata) => {
            let modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
            eprintln!("`{}` last modified: {:?}", lib_file.display(), modified);
            modified
        }
        Err(_) => {
            eprintln!("Library `{}` does not exist, triggering rebuild.", lib_file.display());
            return true; // If `libstarks.a` is missing, we must rebuild.
        }
    };

    // Check if any `.cpp`, `.h`, or `.hpp` file has been modified after `libstarks.a`
    for file in cpp_files {
        if let Ok(metadata) = fs::metadata(file) {
            if let Ok(modified_time) = metadata.modified() {
                if modified_time > lib_modified_time {
                    modified_files.push(file.clone());
                }
            }
        }
    }

    // Print the list of modified files (if any)
    if !modified_files.is_empty() {
        eprintln!("Modified files detected:");
        for file in &modified_files {
            eprintln!(" - {}", file.display());
        }
        return true;
    }

    false // No changes detected
}

/// Finds all `.cpp`, `.h`, and `.hpp` files in `pil2-stark` (recursive search)
fn find_cpp_files(dir: &Path) -> Vec<PathBuf> {
    let mut cpp_files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            // Skip top-level `c/build` and `c/lib` generated outputs. Nested directories
            // that happen to be named build/lib (e.g. `SoftFloat-3e/build/`) contain real
            // headers used by the compilation and must be tracked.
            let name = path.file_name().and_then(|s| s.to_str());
            if dir.ends_with("c") && matches!(name, Some("build" | "lib")) {
                continue;
            }
            if path.is_dir() {
                cpp_files.extend(find_cpp_files(&path));
            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if matches!(ext, "c" | "cpp" | "h" | "hpp" | "S" | "s" | "ld") {
                    cpp_files.push(path);
                }
            }
        }
    }
    cpp_files
}
