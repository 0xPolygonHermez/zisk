//! Spill in-memory Circom templates to temp files. The recurser setup path
//! takes file paths (not strings), so callers holding templates as `String`
//! materialize them here first.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};

/// `None` ⇒ `Ok(None)` so the recurser crate falls back to its built-in defaults.
pub fn write_optional(body: Option<&str>, filename: &str) -> Result<Option<String>> {
    match body {
        None => Ok(None),
        Some(b) => Ok(Some(write_required(b, filename)?)),
    }
}

/// Writes `body` to a temp file and returns its UTF-8 path.
pub fn write_required(body: &str, filename: &str) -> Result<String> {
    let dir = template_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("Failed to create {}", dir.display()))?;
    let path = dir.join(filename);
    std::fs::write(&path, body).with_context(|| format!("Failed to write {}", path.display()))?;
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("template path {} is not valid UTF-8", path.display()))
}

fn template_dir() -> PathBuf {
    // Per-process dir so concurrent processes on the same host don't clobber each other.
    std::env::temp_dir().join(format!("zisk-recurser-tpl-{}", std::process::id()))
}
