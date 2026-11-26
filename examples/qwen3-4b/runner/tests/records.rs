//! Set QWEN_RECORDS to the directory produced by the saved request's prepare command.
use anyhow::{ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::{env, fs, io, path::PathBuf};

#[test]
#[ignore = "requires prepared Qwen3-4B checkpoint records"]
fn matches_frozen_request() -> Result<()> {
    let dir = PathBuf::from(env::var_os("QWEN_RECORDS").context("set QWEN_RECORDS")?);
    let mut total = 0;
    let manifest = include_str!("records.sha256");
    ensure!(manifest.lines().count() == 112 && dir.read_dir()?.count() == 112);
    for entry in manifest.lines() {
        let (expected, name) = entry.split_once("  ").context("invalid SHA-256 manifest")?;
        let mut hash = Sha256::new();
        total += io::copy(&mut fs::File::open(dir.join(name))?, &mut hash)?;
        ensure!(expected == hex::encode(hash.finalize()), "record mismatch in {name}");
    }
    assert_eq!(total, 8_089_899_472);
    Ok(())
}
