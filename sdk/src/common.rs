//! Common types and functions for the ZisK SDK.

use clap::{Parser, ValueEnum};
use colored::Colorize;
use once_cell::sync::Lazy;
use proofman_common::VerboseMode;
use std::borrow::Cow;
use std::env;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use sysinfo::System;
use witness::WitnessLibrary;

pub static DEFAULT_HOME_DIR: Lazy<PathBuf> = Lazy::new(|| PathBuf::from(get_home_dir()));

#[derive(Parser, Debug, Clone, ValueEnum)]
pub enum Field {
    Goldilocks,
    // Add other variants here as needed
}

impl Default for Field {
    fn default() -> Self {
        Field::Goldilocks
    }
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

/// Macro to define a new type around `PathBuf` with a custom default path.
macro_rules! pathbuf_newtype {
    (
        $(#[$outer:meta])*
        $vis:vis $name:ident, $default_expr:expr
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        $vis struct $name(PathBuf);

        impl $name {
            /// Build from any Into<PathBuf>
            $vis fn new(path: Option<impl Into<PathBuf>>) -> Self {
                path.map_or_else($name::default, |p| p.into().into())
            }

            /// Default path inside the user's home directory
            fn default_path() -> PathBuf {
                PathBuf::from($default_expr)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name(Self::default_path())
            }
        }

        impl From<PathBuf> for $name {
            fn from(p: PathBuf) -> Self {
                $name(p)
            }
        }

        impl From<$name> for PathBuf {
            fn from(wrapper: $name) -> PathBuf {
                wrapper.0
            }
        }

        impl AsRef<Path> for $name {
            fn as_ref(&self) -> &Path {
                &self.0
            }
        }
    };
}

pathbuf_newtype! {
    pub WitnessLibPath, format!("{}/.zisk/bin/libzisk_witness.so", get_home_dir())
}

pathbuf_newtype! {
    pub ProvingKeyPath, format!("{}/.zisk/provingKey", get_home_dir())
}

pathbuf_newtype! {
    pub Sha256fScriptPath, format!("{}/.zisk/bin/keccakf_script.json", get_home_dir())
}

pathbuf_newtype! {
    pub OutputPath, "./output"
}

/// PathBufWithDefault is a wrapper around PathBuf that provides a default path if none is provided.
#[derive(Clone)]
pub struct PathBufWithDefault {
    path: PathBuf,
    default_path: PathBuf,
}

impl PathBufWithDefault {
    pub fn new(path: Option<impl Into<PathBuf>>, default_path: impl Into<PathBuf>) -> Self {
        let default_path: PathBuf = default_path.into();
        PathBufWithDefault {
            path: path.map_or_else(|| default_path.clone(), |p| p.into()),
            default_path,
        }
    }

    pub fn set_path(&mut self, path: Option<impl Into<PathBuf>>) {
        self.path = path.map_or_else(|| self.default_path.clone(), |p| p.into().into());
    }

    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.path.to_string_lossy()
    }
}

impl From<PathBufWithDefault> for PathBuf {
    fn from(wrapper: PathBufWithDefault) -> PathBuf {
        wrapper.path
    }
}

impl AsRef<Path> for PathBufWithDefault {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

/// Gets the user's home directory as specified by the HOME environment variable.
pub fn get_home_dir() -> String {
    env::var("HOME").expect("get_home_dir() failed to get HOME environment variable")
}

/// Gets the default zisk folder location in the home installation directory.
pub fn get_home_zisk_path() -> PathBuf {
    let zisk_path = format!("{}/.zisk", get_home_dir());
    PathBuf::from(zisk_path)
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

pub type ZiskLibInitFn<F> = fn(
    VerboseMode,
    PathBuf,         // Rom path
    Option<PathBuf>, // Asm MT path
    Option<PathBuf>, // Asm ROM path
    Option<PathBuf>, // Inputs path
    PathBuf,         // Sha256f script path
) -> Result<Box<dyn WitnessLibrary<F>>, Box<dyn std::error::Error>>;

/// Prints the ZisK and system information.
pub fn print_banner() {
    println!();
    println!(
        "{}",
        format!("{: >12} {}", "ZisK zkVM", env!("CARGO_PKG_VERSION")).bright_purple().bold()
    );

    // System
    let system_name = System::name().unwrap_or_else(|| "<unknown>".to_owned());
    let system_kernel = System::kernel_version().unwrap_or_else(|| "<unknown>".to_owned());
    let system_version = System::long_os_version().unwrap_or_else(|| "<unknown>".to_owned());

    println!(
        "{}",
        format!("{: >12} {} {} ({})", "System", system_name, system_kernel, system_version)
            .bright_green()
            .bold()
    );

    // Hostname
    let system_hostname = System::host_name().unwrap_or_else(|| "<unknown>".to_owned());
    println!("{} {}", format!("{: >12}", "Hostname").bright_green().bold(), system_hostname);

    // CPU
    let system = System::new_all();

    let system_cores = system.cpus().len();
    let system_cores_freq = system.cpus()[0].frequency();
    let system_cores_vendor_id = system.cpus()[0].vendor_id();
    let system_cores_brand = system.cpus()[0].brand();

    let system_cores_desc = format!(
        "{} cores @ {}MHz ({}) {}",
        system_cores, system_cores_freq, system_cores_vendor_id, system_cores_brand
    );
    println!("{} {}", format!("{: >12}", "CPU").bright_green().bold(), system_cores_desc);

    // Memory
    let total_mem = system.total_memory() >> 30;
    let available_mem = system.available_memory() >> 30;
    println!(
        "{} {}GB total ({}GB available)",
        format!("{: >12}", "Mem").bright_green().bold(),
        total_mem,
        available_mem
    );
}
