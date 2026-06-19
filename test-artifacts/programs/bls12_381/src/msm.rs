use ziskos::zisklib::{msm_complete_safe_bls12_381, msm_complete_safe_twist_bls12_381};

use crate::constants::P;

pub fn msm_tests() {
    // Point validation runs before the zero-scalar skip

    // P not in range
    let mut bad_g1 = [0; 12];
    bad_g1[0..6].copy_from_slice(&P);
    let res = msm_complete_safe_bls12_381(&[bad_g1], &[[0; 4]]);
    assert_eq!(res.unwrap_err(), 2);

    // Q not in range
    let mut bad_g2 = [0; 24];
    bad_g2[0..6].copy_from_slice(&P);
    let res = msm_complete_safe_twist_bls12_381(&[bad_g2], &[[0; 4]]);
    assert_eq!(res.unwrap_err(), 2);
}
