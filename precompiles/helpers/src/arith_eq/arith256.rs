// TODO: Implement these functions in assembly to speed things up!

use crate::{bigint_from_u64s, bigint_to_2x4_u64, bigint_to_4_u64};

pub fn arith256(a: &[u64; 4], b: &[u64; 4], c: &[u64; 4], dl: &mut [u64; 4], dh: &mut [u64; 4]) {
    let a = bigint_from_u64s(a);
    let b = bigint_from_u64s(b);
    let c = bigint_from_u64s(c);

    let res = &a * &b + &c;
    bigint_to_2x4_u64(&res, dl, dh);
}

pub fn arith256_mod(a: &[u64; 4], b: &[u64; 4], c: &[u64; 4], module: &[u64; 4], d: &mut [u64; 4]) {
    let a = bigint_from_u64s(a);
    let b = bigint_from_u64s(b);
    let c = bigint_from_u64s(c);
    let module = bigint_from_u64s(module);

    let res = (&a * &b + &c) % &module;
    bigint_to_4_u64(&res, d);
}
