// TODO: Implement these functions in assembly to speed things up!

use crate::arith_eq::{bigint_from_u64s, bigint_to_2x4_u64};

pub fn add256(a: &[u64; 4], b: &[u64; 4], dl: &mut [u64; 4], dh: &mut u64) {
    let a = bigint_from_u64s(a);
    let b = bigint_from_u64s(b);

    let res = &a + &b;
    let mut carry = [0; 4];
    bigint_to_2x4_u64(&res, dl, &mut carry);
    *dh = carry[0];
}

pub fn adc256(a: &[u64; 4], b: &[u64; 4], dl: &mut [u64; 4], dh: &mut u64) {
    let a = bigint_from_u64s(a);
    let b = bigint_from_u64s(b);

    let res = &a + &b + 1;
    let mut carry = [0; 4];
    bigint_to_2x4_u64(&res, dl, &mut carry);
    *dh = carry[0];
}
