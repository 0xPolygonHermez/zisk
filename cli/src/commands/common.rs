use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};
use proofman_common::VerboseMode;
use serde::Deserialize;
use std::env;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use witness::WitnessLibrary;

#[derive(Deserialize)]
struct Metadata {
    target_directory: String,
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Package {
    name: String,
    id: String,
    targets: Vec<Target>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
    crate_types: Vec<String>,
}

#[derive(Parser, Debug, Clone, ValueEnum)]
pub enum Field {
    Goldilocks,
    // Add other variants here as needed
}

impl FromStr for Field {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "goldilocks" => Ok(Field::Goldilocks),
            // Add parsing for other variants here
            _ => Err(format!("'{}' is not a valid value for Field", s)),
        }
    }
}

impl Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Field::Goldilocks => write!(f, "goldilocks"),
        }
    }
}

/// Gets the user's home directory as specified by the HOME environment variable.
pub fn get_home_dir() -> String {
    env::var("HOME").expect("get_home_dir() failed to get HOME environment variable")
}

/// Gets the default witness computation library file location in the home installation directory.
pub fn get_default_witness_computation_lib() -> PathBuf {
    let witness_computation_lib = format!("{}/.zisk/bin/libzisk_witness.so", get_home_dir());
    PathBuf::from(witness_computation_lib)
}

/// Gets the default proving key file location in the home installation directory.
pub fn get_default_proving_key() -> PathBuf {
    let proving_key = format!("{}/.zisk/provingKey", get_home_dir());
    PathBuf::from(proving_key)
}

/// Gets the default stark info JSON file location in the home installation directory.
pub fn get_default_stark_info() -> String {
    let stark_info = format!(
        "{}/.zisk/provingKey/zisk/vadcop_final/vadcop_final.starkinfo.json",
        get_home_dir()
    );
    stark_info
}

/// Gets the default verifier binary file location in the home installation directory.
pub fn get_default_verifier_bin() -> String {
    let verifier_bin =
        format!("{}/.zisk/provingKey/zisk/vadcop_final/vadcop_final.verifier.bin", get_home_dir());
    verifier_bin
}

/// Gets the default verification key JSON file location in the home installation directory.
pub fn get_default_verkey() -> String {
    let verkey =
        format!("{}/.zisk/provingKey/zisk/vadcop_final/vadcop_final.verkey.json", get_home_dir());
    verkey
}

/// Gets the main compiled elf path for the current project and target.
pub fn get_compiled_elf_path(target_triple: &str, release: bool) -> Result<PathBuf> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .map_err(|e| anyhow!("Failed to run cargo metadata: {}", e))?;

    let metadata: Metadata = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow!("Failed to parse cargo metadata output: {}", e))?;

    let root_package_id =
        metadata.workspace_members.first().ok_or_else(|| anyhow!("No workspace members found"))?;

    let package = metadata
        .packages
        .iter()
        .find(|p| &p.id == root_package_id)
        .ok_or_else(|| anyhow!("Failed to find root package in metadata"))?;

    let target = package
        .targets
        .iter()
        .find(|t| t.kind.contains(&"bin".to_string()))
        .ok_or_else(|| anyhow!("Failed to find binary target in package"))?;

    let mut path = PathBuf::from(&metadata.target_directory);
    path.push(target_triple);
    path.push(if release { "release" } else { "debug" });
    path.push(&target.name);

    if !path.exists() {
        return Err(anyhow!("Compiled binary not found at expected path: {}", path.display()));
    }

    Ok(path)
}

pub type ZiskLibInitFn<F> = fn(
    VerboseMode,
    PathBuf,         // Rom path
    Option<PathBuf>, // Asm path
    Option<PathBuf>, // Inputs path
    PathBuf,         // Keccak path
) -> Result<Box<dyn WitnessLibrary<F>>, Box<dyn std::error::Error>>;
