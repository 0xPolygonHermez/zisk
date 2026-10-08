//! Execution-size parameters shared by the executor and the constraints.

use zisk_definitions_macros::constants;

#[constants(group = "execution", to(rust, c, pil), hex)]
pub mod execution {
    /// log2 of the maximum step count. Reads better in decimal.
    #[emit(dec)]
    pub const MAIN_STEP_BITS: u32 = 36;

    /// Maximum number of execution steps (a hard PIL constraint).
    /// Exceeds 32 bits, so widen the inherited fit check.
    #[emit(fits = 64)]
    pub const MAX_STEPS: u64 = 1u64 << MAIN_STEP_BITS;
}

pub use execution::{EXPORTS, GROUP};
