use std::sync::Arc;

use p3_field::PrimeField;
use ziskemu::EmuTrace;

use crate::{Plan, WitnessBuffer};

pub trait Expander<'a, F: PrimeField> {
    fn expand(
        &self,
        plan: &Plan,
        min_traces: Arc<[EmuTrace]>,
        buffer: WitnessBuffer<'a, F>,
    ) -> Result<(), Box<dyn std::error::Error + Send>>;
}
