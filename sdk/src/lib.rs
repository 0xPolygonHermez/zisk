mod builder;
mod client;
mod prover;
mod utils;
mod verifier;
mod ziskemu;

pub use builder::*;
pub use client::ProverClient;
pub use prover::*;
pub use utils::*;
pub use verifier::*;

pub use ziskemu::*;

pub use proofman_common::VerboseMode;

pub use zisk_common::{io::*, ElfBinary, ElfBinaryFromFile};

pub use zisk_build::*;

#[macro_export]
macro_rules! include_elf {
    ($arg:literal) => {{
        const WITH_HINTS: bool = option_env!(concat!("ZISK_ELF_", $arg, "_WITH_HINTS")).is_some();

        ElfBinary {
            elf: include_bytes!(env!(concat!("ZISK_ELF_", $arg))),
            name: $arg,
            with_hints: WITH_HINTS,
            path: Some(env!(concat!("ZISK_ELF_", $arg))),
        }
    }};
}
