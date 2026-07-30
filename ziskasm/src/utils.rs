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
    let is_zisk = |p: &Path| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zisk"));

    let p = Path::new(path);
    let metadata =
        std::fs::metadata(p).map_err(|_| format!("ZisK assembly path '{path}' does not exist"))?;

    if metadata.is_dir() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(p)
            .map_err(|e| format!("Could not read directory '{path}': {e}"))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && is_zisk(p))
            .collect();
        if files.is_empty() {
            return Err(format!("directory '{path}' contains no .zisk files"));
        }
        files.sort();
        Ok(files)
    } else if is_zisk(p) {
        Ok(vec![p.to_path_buf()])
    } else {
        Err(format!("ZisK assembly file '{path}' must have the .zisk extension"))
    }
}
