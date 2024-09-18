use std::ops::Add;

const ARITH_RANGE_TABLE_SIZE: usize = 2 << 17;

pub struct ArithRangeTableInputs<F> {
    multiplicity: [u32; ARITH_RANGE_TABLE_SIZE],
    _phantom: std::marker::PhantomData<F>,
}

impl<F> Add for ArithRangeTableInputs<F> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        let mut result = Self::new();
        for i in 0..ARITH_RANGE_TABLE_SIZE {
            result.multiplicity[i] = self.multiplicity[i] + other.multiplicity[i];
        }
        result
    }
}

impl<F> ArithRangeTableInputs<F> {
    pub fn new() -> Self {
        Self { multiplicity: [0; ARITH_RANGE_TABLE_SIZE], _phantom: std::marker::PhantomData }
    }
    pub fn clear(&mut self) {
        self.multiplicity = [0; ARITH_RANGE_TABLE_SIZE];
    }
    pub fn push(&mut self, range_id: u8, value: u64) {
        Self::check_value(range_id, value);
        self.fast_push(range_id, value);
    }
    fn get_row(range_id: u8, value: u64) -> usize {
        usize::try_from(value + if range_id > 0 { 2 << 16 } else { 0 }).unwrap() %
            ARITH_RANGE_TABLE_SIZE
    }
    fn check_value(range_id: u8, value: u64) {
        match range_id {
            0 => assert!(value <= 0xFFFF),
            1 => assert!(value <= 0x7FFF),
            2 => assert!(value <= 0xFFFF && value >= 0x8000),
            _ => assert!(false),
        };
    }

    pub fn fast_push(&mut self, op: u8, value: u64) {
        self.multiplicity[Self::get_row(op, value)] += 1;
    }
}
