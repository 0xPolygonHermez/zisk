use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

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
    let library_name = "ziskfloat";
    let lib_file = library_folder.join(format!("lib{library_name}.a"));
    let elf_file = library_folder.join("ziskfloat.elf");

    fs::create_dir_all(&build_folder)
        .unwrap_or_else(|e| panic!("Failed to create build directory: {e}"));
    fs::create_dir_all(&library_folder)
        .unwrap_or_else(|e| panic!("Failed to create lib directory: {e}"));

    // Track sources first so we can decide whether a rebuild is needed.
    let source_files = find_source_files(&c_path);
    eprintln!("Tracking {} source files", source_files.len());
    for file in &source_files {
        println!("cargo:rerun-if-changed={}", file.display());
    }
    println!("cargo:rerun-if-changed={}", c_path.join("Makefile").display());

    if needs_rebuild(&source_files, &lib_file) {
        eprintln!("Building lib-float into {}", out_dir.display());
        run_command(
            "make",
            &[
                &format!("BUILD_DIR={}", build_folder.display()),
                &format!("LIB_DIR={}", library_folder.display()),
            ],
            &c_path,
        );
    } else {
        eprintln!("lib-float artifacts up to date, skipping rebuild");
    }

    if !lib_file.exists() {
        panic!("`{}` was not found after build", lib_file.display());
    }
    if !elf_file.exists() {
        panic!("`{}` was not found after build", elf_file.display());
    }

    // Publish the artifacts at a stable workspace-relative location so other
    // tooling (and downstream crates that don't depend on us via `links`) can
    // find them. Living under target/ means `cargo clean` removes it too.
    let runtime_dir = workspace_target_dir(&out_dir).join("zisk-libs");
    fs::create_dir_all(&runtime_dir)
        .unwrap_or_else(|e| panic!("Failed to create runtime lib dir: {e}"));
    let runtime_lib = runtime_dir.join(format!("lib{library_name}.a"));
    let runtime_elf = runtime_dir.join("ziskfloat.elf");
    fs::copy(&lib_file, &runtime_lib).unwrap_or_else(|e| {
        panic!("Failed to copy {} to {}: {e}", lib_file.display(), runtime_lib.display())
    });
    fs::copy(&elf_file, &runtime_elf).unwrap_or_else(|e| {
        panic!("Failed to copy {} to {}: {e}", elf_file.display(), runtime_elf.display())
    });

    // Expose the ELF path to dependent crates' build scripts as
    // `DEP_ZISKFLOAT_ELF_PATH` (cargo passes `cargo:KEY=VALUE` from a
    // `links`-tagged crate as `DEP_<links_uppercased>_<KEY>`).
    println!("cargo:lib_dir={}", library_folder.display());
    println!("cargo:elf_path={}", elf_file.display());
    println!("cargo:runtime_lib_dir={}", runtime_dir.display());

    // Link the static library into our own crate.
    println!("cargo:rustc-link-search=native={}", library_folder.display());
    println!("cargo:rustc-link-lib=static={library_name}");
    for lib in &["pthread", "gmp", "stdc++", "gmpxx", "c"] {
        println!("cargo:rustc-link-lib={lib}");
    }
}

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

fn needs_rebuild(cpp_files: &[PathBuf], lib_file: &Path) -> bool {
    let lib_modified_time = match fs::metadata(lib_file) {
        Ok(metadata) => metadata.modified().unwrap_or(UNIX_EPOCH),
        Err(_) => return true,
    };
    cpp_files.iter().any(|file| {
        fs::metadata(file)
            .and_then(|m| m.modified())
            .map(|t| t > lib_modified_time)
            .unwrap_or(false)
    })
}

fn find_source_files(dir: &Path) -> Vec<PathBuf> {
    fn is_tracked_ext(ext: &std::ffi::OsStr) -> bool {
        matches!(ext.to_str(), Some("c" | "cpp" | "h" | "hpp" | "S" | "s" | "asm" | "ld"))
    }
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(find_source_files(&path));
        } else if let Some(ext) = path.extension() {
            if is_tracked_ext(ext) {
                files.push(path);
            }
        }
    }
    files
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
