use p3_field::AbstractField;
use zisk_core::ZiskRequired;

use crate::EmuFullTraceStep;

pub struct EmuSlice<F: AbstractField> {
    pub full_trace: Vec<EmuFullTraceStep<F>>,
    pub required: ZiskRequired,
}

//implment a new function with a Vec size as an argument
impl<F: AbstractField> EmuSlice<F> {
    pub fn new(size: usize) -> Self {
        EmuSlice { full_trace: Vec::with_capacity(size), required: ZiskRequired::default() }
    }
}
