include!("../bindings.rs");

pub fn add_point_ec_c(
    dbl: u64,
    x1: &[u64; 2],
    y1: &[u64; 2],
    x2: &[u64; 2],
    y2: &[u64; 2],
    x3: &mut [u64; 2],
    y3: &mut [u64; 2],
) -> i32 {
    unsafe { AddPointEc(dbl, &x1[0], &y1[0], &x2[0], &y2[0], &mut x3[0], &mut y3[0]) }
}
