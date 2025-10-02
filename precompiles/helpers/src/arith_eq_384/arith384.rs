// TODO: Implement these functions in assembly to speed things up!

use crate::{bigint_from_u64s, bigint_to_6_u64};

pub fn arith384_mod(a: &[u64; 6], b: &[u64; 6], c: &[u64; 6], module: &[u64; 6], d: &mut [u64; 6]) {
    let a = bigint_from_u64s(a);
    let b = bigint_from_u64s(b);
    let c = bigint_from_u64s(c);
    let module = bigint_from_u64s(module);

    let res = (&a * &b + &c) % &module;
    bigint_to_6_u64(&res, d);
}
