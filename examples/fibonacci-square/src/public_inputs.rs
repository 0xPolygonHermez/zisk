use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct FibonacciSquarePublics {
    pub module: u64,
    pub a: u64,
    pub b: u64,
}
