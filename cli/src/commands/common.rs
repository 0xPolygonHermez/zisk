use anyhow::Result;
use clap::{Parser, ValueEnum};
#[cfg(distributed)]
use mpi::traits::*;
use std::env;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use zisk_common::MpiContext;

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
            _ => Err(format!("'{s}' is not a valid value for Field")),
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

/// Gets the default zisk folder location in the home installation directory.
pub fn get_home_zisk_path() -> PathBuf {
    let zisk_path = format!("{}/.zisk", get_home_dir());
    PathBuf::from(zisk_path)
}

/// Gets the default zisk folder location in the home installation directory.
pub fn get_default_zisk_path() -> PathBuf {
    let zisk_path = format!("{}/.zisk/zisk", get_home_dir());
    PathBuf::from(zisk_path)
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

/// If the target_os is macOS returns an error indicating that the command is not supported.
pub fn cli_fail_if_macos() -> anyhow::Result<()> {
    if cfg!(target_os = "macos") {
        Err(anyhow::anyhow!("Command is not supported on macOS"))
    } else {
        Ok(())
    }
}

/// If the feature "gpu" is enabled, returns an error indicating that the command is not supported.
pub fn cli_fail_if_gpu_mode() -> anyhow::Result<()> {
    if cfg!(feature = "gpu") {
        Err(anyhow::anyhow!("Command is not supported on GPU mode"))
    } else {
        Ok(())
    }
}

#[cfg(distributed)]
pub fn initialize_mpi() -> Result<MpiContext> {
    let (universe, _threading) = mpi::initialize_with_threading(mpi::Threading::Multiple)
        .ok_or_else(|| anyhow::anyhow!("Failed to initialize MPI with threading"))?;

    let world = universe.world();
    let world_rank = world.rank();

    let local_comm = world.split_shared(world_rank);
    let local_rank = local_comm.rank();

    Ok(MpiContext { universe, world_rank, local_rank })
}

#[cfg(not(distributed))]
pub fn initialize_mpi() -> Result<MpiContext> {
    Ok(MpiContext { world_rank: 0, local_rank: 0 })
}

/// Gets the witness computation library file location.
/// Uses the default one if not specified by user.
pub fn get_witness_computation_lib(witness_lib: Option<&PathBuf>) -> PathBuf {
    witness_lib.cloned().unwrap_or_else(get_default_witness_computation_lib)
}

/// Gets the proving key file location.
/// Uses the default one if not specified by user.
pub fn get_proving_key(proving_key: Option<&PathBuf>) -> PathBuf {
    proving_key.cloned().unwrap_or_else(get_default_proving_key)
}

/// Gets the zisk folder.
/// Uses the default one if not specified by user.
pub fn get_zisk_path(zisk_path: Option<&PathBuf>) -> PathBuf {
    zisk_path.cloned().unwrap_or_else(get_default_zisk_path)
}
