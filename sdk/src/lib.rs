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

pub struct Proof {
    pub id: String,
    pub proof: VadcopFinalProof,
}

#[macro_export]
macro_rules! include_elf {
    ($arg:tt) => {{
        include_bytes!(env!(concat!("ZISK_ELF_", $arg)))
    }};
}
