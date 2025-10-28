use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

fn main() {
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-cfg=feature=\"no_lib_link\"");
        return;
    }

    // // **Check if the `no_lib_link` feature is enabled**
    // if env::var("CARGO_FEATURE_NO_LIB_LINK").is_ok() {
    //     println!("Skipping linking because `no_lib_link` feature is enabled.");
    //     return;
    // }

    // Paths
    let c_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("c");
    if !c_path.exists() {
        panic!("Missing c_path = {}", c_path.display());
    }
    let library_folder = c_path.join("lib");
    let library_name = "ziskfloat";
    let lib_file = library_folder.join(format!("lib{library_name}.a"));

    // Check if the C++ library exists before recompiling
    if !lib_file.exists() {
        println!("`{}` not found! Compiling...", lib_file.display());
        run_command("make", &["clean"], &c_path);
        run_command("make", &[], &c_path);
    } else {
        println!("C++ library already compiled, skipping rebuild.");
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
        .status()
        .unwrap_or_else(|e| panic!("Failed to execute `{cmd}`: {e}"));

    if !status.success() {
        panic!("Command `{}` failed with exit code {:?}", cmd, status.code());
    }
}

/// Tracks changes in the `pil2-stark` directory to trigger recompilation only when needed
fn track_cpp_changes(c_path: &Path) {
    let cpp_files = find_cpp_files(c_path);
    let lib_file = c_path.join("lib/libziskfloat.a");

    // Print tracked files for debugging
    eprintln!("Tracking {} C++ source files:", cpp_files.len());
    for file in &cpp_files {
        eprintln!(" - {}", file.display());
        println!("cargo:rerun-if-changed={}", file.display());
    }

    // If any C++ source file changed, force a rebuild
    if cpp_files_have_changed(&cpp_files, &lib_file) {
        eprintln!("Changes detected! Running `make clean` and recompiling...");
        run_command("make", &["clean"], c_path);
        run_command("make", &[], c_path);
    } else {
        println!("No C++ source changes detected, skipping rebuild.");
    }
}
/// Checks if any `.cpp`, `.h`, or `.hpp` file has changed since the last build
fn cpp_files_have_changed(cpp_files: &[PathBuf], lib_file: &Path) -> bool {
    let mut modified_files: Vec<PathBuf> = Vec::new();

    // Get the modification time of `libstarks.a`
    let lib_modified_time = match fs::metadata(lib_file) {
        Ok(metadata) => {
            let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
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
            if path.is_dir() {
                cpp_files.extend(find_cpp_files(&path));
            } else if let Some(ext) = path.extension() {
                if (ext == "cpp" || ext == "h" || ext == "hpp")
                    && path.file_name() != Some(std::ffi::OsStr::new("starks_lib.h"))
                {
                    cpp_files.push(path);
                }
            }
        }
    }
    cpp_files
}
