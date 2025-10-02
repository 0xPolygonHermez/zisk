use std::path::PathBuf;

use proofman_common::VerboseMode;
use witness::WitnessLibrary;

pub type ZiskLibInitFn<F> =
    fn(
        VerboseMode,
        PathBuf,         // Rom path
        Option<PathBuf>, // Asm path
        Option<PathBuf>, // Asm ROM path
        Option<i32>,     // mpi World Rank
        Option<i32>,     // mpi Local Rank
        Option<u16>,     // Base port for the ASM microservices
        bool,            // Unlock_mapped_memory
        bool,            // Shared_tables
    ) -> Result<Box<dyn WitnessLibrary<F> + Send + Sync>, Box<dyn std::error::Error>>;
