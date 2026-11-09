//! Build native Nitro helpers and prepare recorded replay inputs.
mod build;
mod witness;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Build the pinned native C helper.
    Build(build::Args),
    /// Encode a recorded transition and its separate expected public claim.
    Witness(witness::Args),
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Action::Build(args) => build::build(args),
        Action::Witness(args) => witness::prepare(args),
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn runtime() -> PathBuf {
    root().join("transpilers/wasm/runtime")
}

fn run(command: &mut Command) -> Result<Output> {
    let output = command.output().with_context(|| format!("start {command:?}"))?;
    if !output.status.success() {
        bail!(
            "{command:?}: {}\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output)
}

fn text(command: &mut Command) -> Result<String> {
    Ok(String::from_utf8(run(command)?.stdout)?.trim().to_owned())
}

fn sha(path: &Path) -> Result<String> {
    Ok(hex::encode(Sha256::digest(fs::read(path)?)))
}

fn check_sha(path: &Path, expected: &str) -> Result<()> {
    anyhow::ensure!(sha(path)? == expected, "checksum mismatch: {}", path.display());
    Ok(())
}

fn write_json(path: &Path, value: &serde_json::Value) -> Result<()> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

fn tool(variable: &str, candidates: &[&str]) -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(variable) {
        return Ok(path.into());
    }
    for name in candidates {
        let path = Path::new(name);
        if path.is_absolute() && path.is_file() {
            return Ok(path.into());
        }
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }
    }
    bail!("set {variable} to a suitable tool ({})", candidates.join(", "))
}

fn sources(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut paths =
        fs::read_dir(dir)?.map(|e| e.map(|e| e.path())).collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|p| p.extension().is_some_and(|e| e == extension));
    paths.sort();
    Ok(paths)
}

fn output_dir(path: Option<PathBuf>, default: &str) -> Result<PathBuf> {
    // Builders replace their own generated subdirectories; never allow that outside target/.
    let target = root().join("target");
    fs::create_dir_all(&target)?;
    let target = target.canonicalize()?;
    let path = std::path::absolute(path.unwrap_or_else(|| target.join(default)))?;
    anyhow::ensure!(
        path.starts_with(&target)
            && path != target
            && !path.components().any(|part| part == std::path::Component::ParentDir),
        "--out must be a subdirectory of this worktree's target directory"
    );
    let ancestor = path.ancestors().find(|p| p.exists()).context("missing output ancestor")?;
    anyhow::ensure!(
        ancestor.canonicalize()?.starts_with(&target),
        "--out escapes target through a symlink"
    );
    fs::create_dir_all(&path)?;
    let path = path.canonicalize()?;
    anyhow::ensure!(
        path.starts_with(&target) && path != target,
        "--out must be a subdirectory of this worktree's target directory"
    );
    Ok(path)
}

fn fresh_dir(path: &Path) -> Result<()> {
    if path.symlink_metadata().is_ok() {
        anyhow::ensure!(
            !path.symlink_metadata()?.file_type().is_symlink(),
            "refusing to replace symlink {}",
            path.display()
        );
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn git_archive(repo: &Path, revision: &str, destination: &Path) -> Result<()> {
    let actual = text(
        Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["rev-parse", &format!("{revision}^{{commit}}")]),
    )?;
    anyhow::ensure!(actual == revision, "wrong source revision: {actual}");
    fs::create_dir_all(destination)?;
    let archive = destination.join(".source.tar");
    run(Command::new("git").arg("-C").arg(repo).args(["archive", revision, "-o"]).arg(&archive))?;
    run(Command::new("tar").arg("-xf").arg(&archive).arg("-C").arg(destination))?;
    fs::remove_file(archive)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_output_outside_target_before_creating_it() {
        let outside = root().join(format!("nitro-invalid-output-{}", std::process::id()));
        assert!(!outside.exists());
        assert!(output_dir(Some(outside.clone()), "unused").is_err());
        assert!(!outside.exists());
        assert!(output_dir(Some(root().join("target")), "unused").is_err());
        assert!(output_dir(Some(root().join("target/../outside")), "unused").is_err());
    }
}
