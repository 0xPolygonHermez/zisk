//! The proving-key (setup) version this build requires, decoupled from the crate version. Set in
//! `setup/Cargo.toml`, written into `pilout.globalInfo.json` as `setupVersion` by
//! `proofman-setup setup`, checked before proving.

use std::io::{Error, Result};
use std::path::Path;

pub const ZISK_SETUP_VERSION: &str = env!("ZISK_SETUP_VERSION");

/// Fails unless the key at `proving_key` was built for [`ZISK_SETUP_VERSION`]. A key without
/// `setupVersion` is rejected too.
pub fn check_setup_version(proving_key: &Path) -> Result<()> {
    check(proving_key, ZISK_SETUP_VERSION)
}

fn check(proving_key: &Path, expected: &str) -> Result<()> {
    let path = proving_key.join("pilout.globalInfo.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| Error::other(format!("failed to read {}: {e}", path.display())))?;
    let info: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| Error::other(format!("failed to parse {}: {e}", path.display())))?;
    match info.get("setupVersion").and_then(|v| v.as_str()) {
        Some(v) if v == expected => Ok(()),
        found => Err(Error::other(format!(
            "proving key at {} has setup version {}, but this ZisK build requires {expected}. \
             Install the matching key with `ziskup`, or regenerate it with \
             `cargo-zisk-dev proofman-setup setup`",
            proving_key.display(),
            found.unwrap_or("<none>"),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::check;

    fn key(name: &str, global_info: Option<&str>) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("zisk-setup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        if let Some(json) = global_info {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("pilout.globalInfo.json"), json).unwrap();
        }
        dir
    }

    #[test]
    fn exact_version_only() {
        let dir = key("exact", Some(r#"{"setupVersion":"1.3.1"}"#));
        assert!(check(&dir, "1.3.1").is_ok());
        assert!(check(&dir, "1.3.2").is_err());
    }

    #[test]
    fn rejects_missing_field_or_key() {
        let msg = check(&key("nofield", Some("{}")), "1.3.1").unwrap_err().to_string();
        assert!(msg.contains("<none>") && msg.contains("1.3.1"), "{msg}");
        assert!(check(&key("absent", None), "1.3.1").is_err());
    }
}
