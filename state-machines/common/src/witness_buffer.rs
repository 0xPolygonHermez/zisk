use p3_field::PrimeField;

pub struct WitnessBuffer<F: PrimeField> {
    pub buffer: Vec<F>,
    pub offset: u64,
}

impl<F: PrimeField> WitnessBuffer<F> {
    pub fn new(buffer: Vec<F>, offset: u64) -> Self {
        Self { buffer, offset }
    }
}
