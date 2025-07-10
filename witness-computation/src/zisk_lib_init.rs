use std::path::PathBuf;

use proofman_common::VerboseMode;
use witness::WitnessLibrary;

#[cfg(feature = "dev")]
use executor::DynSMBundle;
#[cfg(feature = "dev")]
use pil_std_lib::Std;
#[cfg(feature = "dev")]
use std::sync::Arc;

#[cfg(feature = "dev")]
pub type ZiskLibInitFn<F> =
    fn(
        VerboseMode,
        PathBuf,         // Rom path
        Option<PathBuf>, // Asm path
        Option<PathBuf>, // Asm ROM path
        PathBuf,         // Sha256f script path
        Option<u64>,     // Chunk size
        Option<i32>,     // mpi World Rank
        Option<i32>,     // mpi Local Rank
        Option<u16>,     // Base port for the ASM microservices
        bool,            // Unlock_mapped_memory
        fn(
            Arc<witness::WitnessManager<F>>,
            Arc<Std<F>>,
            Arc<zisk_core::ZiskRom>,
            Option<PathBuf>,
            PathBuf,
        ) -> (DynSMBundle<F>, bool),
    ) -> Result<Box<dyn WitnessLibrary<F> + Send + Sync>, Box<dyn std::error::Error>>;

#[cfg(all(not(feature = "dev"), not(feature = "unit")))]
pub type ZiskLibInitFn<F> =
    fn(
        VerboseMode,
        PathBuf,         // Elf path
        Option<PathBuf>, // Asm path
        Option<PathBuf>, // Asm ROM path
        PathBuf,         // Sha256f script path
        Option<u64>,     // Chunk size
        Option<i32>,     // mpi World Rank
        Option<i32>,     // mpi Local Rank
        Option<u16>,     // Base port for the ASM microservices
        bool,            // Unlock_mapped_memory
    ) -> Result<Box<dyn WitnessLibrary<F> + Send + Sync>, Box<dyn std::error::Error>>;

#[cfg(all(not(feature = "dev"), feature = "unit"))]
pub type ZiskLibInitFn<F> =
    fn(
        VerboseMode,
        Option<PathBuf>, // Asm path
        Option<PathBuf>, // Asm ROM path
        PathBuf,         // Sha256f script path
        Option<u64>,     // Chunk size
        Option<i32>,     // mpi World Rank
        Option<i32>,     // mpi Local Rank
        Option<u16>,     // Base port for the ASM microservices
        bool,            // Unlock_mapped_memory
    ) -> Result<Box<dyn WitnessLibrary<F> + Send + Sync>, Box<dyn std::error::Error>>;
