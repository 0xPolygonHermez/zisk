use p3_field::AbstractField;

use crate::{EmuFullTraceStep, EmuRequired};

pub struct EmuSlice<F: AbstractField> {
    pub full_trace: Vec<EmuFullTraceStep<F>>,
    pub required: EmuRequired,
}
