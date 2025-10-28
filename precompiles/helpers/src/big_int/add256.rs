// TODO: Implement these functions in assembly to speed things up!

use crate::{bigint_from_u64s, bigint_to_4_u64_with_cout};

pub fn add256(a: &[u64; 4], b: &[u64; 4], cin: u64, c: &mut [u64; 4]) -> u64 {
    let a = bigint_from_u64s(a);
    let b = bigint_from_u64s(b);
    debug_assert!(cin <= 1);

    let mut res = &a + &b;
    if cin == 1 {
        res += 1;
    }
    bigint_to_4_u64_with_cout(&res, c)
}
