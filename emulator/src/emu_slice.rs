use p3_field::AbstractField;
use zisk_core::ZiskRequired;

use crate::EmuFullTraceStep;

pub struct EmuSlice<F: AbstractField> {
    pub full_trace: Vec<EmuFullTraceStep<F>>,
    pub required: ZiskRequired,
}
