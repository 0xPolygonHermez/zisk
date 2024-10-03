mod air_instance;
mod air_instances_repository;
mod buffer_allocator;
mod verbose_mode;
mod execution_ctx;
mod lib_pilout;
mod proof_ctx;
mod prover;
mod extended_field;
mod setup;
mod setup_ctx;
pub mod trace;
pub mod global_info;

pub use air_instance::*;
pub use air_instances_repository::*;
pub use buffer_allocator::*;
use proofman_starks_lib_c::set_log_level_c;
pub use verbose_mode::*;
pub use execution_ctx::*;
pub use lib_pilout::*;
pub use proof_ctx::*;
pub use prover::*;
pub use extended_field::*;
pub use global_info::*;
pub use setup::*;
pub use setup_ctx::*;

pub fn initialize_logger(verbose_mode: VerboseMode) {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(verbose_mode.into())
        .init();
    set_log_level_c(verbose_mode.into());
}
