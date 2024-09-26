pub struct StdRangeCheckOp (u64, u64, u64);

#[derive(Debug, Clone)]
pub enum StdOp<F, R> {
    RangeCheck(F, R, R),
}

pub enum StdOpResult {
    None,
}
