mod builder;
mod client;
mod prover;
mod utils;
mod zisk_lib_loader;

pub use builder::*;
pub use client::ProverClient;
pub use proofman_util::VadcopFinalProof;
pub use prover::*;
pub use utils::*;
pub use zisk_lib_loader::*;

pub struct RankInfo {
    pub world_rank: i32,
    pub local_rank: i32,
}

#[macro_export]
macro_rules! include_elf {
    ($arg:literal) => {{
        const WITH_HINTS: bool = option_env!(concat!("ZISK_ELF_", $arg, "_WITH_HINTS")).is_some();

        ElfBinary {
            elf: include_bytes!(env!(concat!("ZISK_ELF_", $arg))),
            name: $arg,
            with_hints: WITH_HINTS,
        }
    }};
}
