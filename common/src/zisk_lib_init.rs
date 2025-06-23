use std::path::PathBuf;

use proofman_common::VerboseMode;
use witness::WitnessLibrary;

pub type ZiskLibInitFn<F> = fn(
    VerboseMode,
    PathBuf,         // Rom path
    Option<PathBuf>, // Asm path
    Option<PathBuf>, // Asm ROM path
    PathBuf,         // Sha256f script path
    Option<i32>,     // mpi World Rank
    Option<i32>,     // mpi Local Rank
    Option<u16>,     // Port for the server
) -> Result<Box<dyn WitnessLibrary<F>>, Box<dyn std::error::Error>>;
