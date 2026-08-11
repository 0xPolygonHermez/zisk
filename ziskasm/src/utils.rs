//! Small helpers shared by the `ziskasm` library, its binaries, and the emulator.

use std::path::{Path, PathBuf};

/// Resolves a `.zisk` input argument into the list of files to assemble.
///
/// If `path` is a single file it must have the `.zisk` extension. If it is a
/// directory, every `.zisk` file directly inside it is collected, sorted by name
/// for a deterministic assembly order (labels are resolved globally, so the order
/// only affects instruction addresses, not correctness).
///
/// Errors are returned as plain strings so callers can wrap them in whatever
/// error type they use.
pub fn collect_zisk_files(path: &str) -> Result<Vec<PathBuf>, String> {
    let p = Path::new(path);
    let metadata =
        std::fs::metadata(p).map_err(|_| format!("ZisK assembly path '{path}' does not exist"))?;

    if metadata.is_dir() {
        // Collect `.zisk` files from the directory *and all subdirectories*, so a
        // library can be organised into per-family folders (e.g. `uint256/`).
        let mut files: Vec<PathBuf> = Vec::new();
        collect_zisk_files_rec(p, &mut files)
            .map_err(|e| format!("Could not read directory '{path}': {e}"))?;
        if files.is_empty() {
            return Err(format!("directory '{path}' contains no .zisk files"));
        }
        // Sort by path for a deterministic assembly order (the assembler still
        // moves the file defining `_start`/`main` to the front for programs).
        files.sort();
        Ok(files)
    } else if is_zisk_file(p) {
        Ok(vec![p.to_path_buf()])
    } else {
        Err(format!("ZisK assembly file '{path}' must have the .zisk extension"))
    }
}

/// True if `p` has a (case-insensitive) `.zisk` extension.
fn is_zisk_file(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zisk"))
}

/// Recursively appends every `.zisk` file under `dir` to `out`.
fn collect_zisk_files_rec(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_zisk_files_rec(&path, out)?;
        } else if path.is_file() && is_zisk_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}
