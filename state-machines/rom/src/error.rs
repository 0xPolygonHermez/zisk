//! Crate-local error type for the ROM state machine.

use thiserror::Error;

/// Errors produced by this crate.
#[derive(Debug, Error)]
pub enum RomError {
    /// The ELF could not be transpiled into a `ZiskRom`.
    #[error("failed to transpile ELF to ZiskRom: {0}")]
    ElfTranspile(String),

    /// The transpiled ROM has more instructions than the custom ROM trace can hold.
    #[error("the generated ROM has {len} instructions, which exceeds the maximum supported by the custom ROM trace ({max_len} instructions); please review zisk.pil and increase the ROM trace size accordingly")]
    RomTooLarge {
        /// Number of instructions in the transpiled ROM.
        len: usize,
        /// Maximum number of rows the custom ROM trace can hold.
        max_len: usize,
    },

    /// The trace buffer could not be wrapped into a `RomRomTrace`.
    #[error("failed to construct custom ROM trace: {0}")]
    TraceConstruction(String),

    /// [`RomSM::set_rom`](crate::RomSM::set_rom) was called more than once.
    #[error("RomSM::set_rom called more than once")]
    RomAlreadySet,

    /// The internal mutex protecting the assembly-runner histogram is poisoned (another
    /// thread panicked while holding it).
    #[error("RomSM rh_data mutex poisoned")]
    RhDataPoisoned,

    /// A collector dispatched to the ROM AIR was not a `RomCollector` as expected —
    /// a framework-side invariant violation.
    #[error("collector dispatched to ROM AIR is not a RomCollector")]
    BadCollectorType,
}

/// Convenience [`Result`] alias for fallible operations in this crate.
pub type RomResult<T> = Result<T, RomError>;
