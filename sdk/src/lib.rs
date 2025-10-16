mod builder;
mod client;
mod prover;
mod utils;
mod zisk_lib_loader;

pub use builder::*;
pub use client::ProverClient;
pub use utils::*;
pub use zisk_lib_loader::*;

pub struct RankInfo {
    pub world_rank: i32,
    pub local_rank: i32,
}

pub struct Proof {
    pub id: Option<String>,
    pub proof: Option<Vec<u64>>,
}
