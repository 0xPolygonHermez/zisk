//! Emulator slice.  
//! After executing the program during the witness computation, the main state machine generates
//! two data sets:
//! * The trace of field elements required to prove the main state machine operations
//! * The data required to prove the operations delegated to the secondary state machines

use p3_field::AbstractField;
use zisk_core::ZiskRequired;

use crate::EmuFullTraceStep;

/// Emulator slice
pub struct EmuSlice<F: AbstractField> {
    /// Vector of field element traces, one per step, required to prove the main state machine
    /// operations
    pub full_trace: Vec<EmuFullTraceStep<F>>,
    /// Data required to prove the operations delegated to the secondary state machines
    pub required: ZiskRequired,
}

impl<F: AbstractField> EmuSlice<F> {
    /// Constructor of EmuSlice based on the size of the vector
    pub fn new(size: usize) -> Self {
        EmuSlice { full_trace: Vec::with_capacity(size), required: ZiskRequired::default() }
    }
}
