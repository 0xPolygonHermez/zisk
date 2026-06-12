use ziskos::zisklib::{
    double_scalar_mul_with_g_secp256k1, glv_double_scalar_mul_with_g_secp256k1,
    glv_scalar_mul_secp256k1, is_on_curve_secp256k1, scalar_mul_secp256k1,
};

use crate::constants::{G, G_NEG, IDENTITY};

pub fn curve_tests() {
    // Is on curve
    let p = IDENTITY;
    let res = is_on_curve_secp256k1(&p);
    assert_eq!(res, true);

    let p = G;
    let res = is_on_curve_secp256k1(&p);
    assert_eq!(res, true);

    // Scalar multiplication
    let s = [0x53abb24c9f99136, 0x3c3ba88ce76f629e, 0xe9056abea1783a93, 0x6841f6b8ac6be1d];
    let p = G;
    let res1 = match scalar_mul_secp256k1(&s, &p) {
        Some(point) => point,
        None => panic!("Scalar multiplication failed"),
    };
    let res2 = match glv_scalar_mul_secp256k1(&s, &p) {
        Some(point) => point,
        None => panic!("GLV scalar multiplication failed"),
    };
    let res_exp = [
        0xca9cbc09c949b822,
        0xcf7cd8156abf9fc2,
        0x493d8feaee4890a5,
        0xb6da83ed96d53183,
        0x9ba9c4d02851f15f,
        0xc59ee8217ea643ed,
        0x92738c7c8ee06f97,
        0x4f32ad5172c6f18b,
    ];
    assert_eq!(res1, res_exp);
    assert_eq!(res2, res_exp);

    // Double scalar multiplication with G
    let k1 = [0xf447e442a44c829e, 0xe979220cfe9824d3, 0x673913d78b5bdbfe, 0xd961172287f69999];
    let k2 = [0x9249014d999485b7, 0xdfd89459c31678cb, 0x4436e3fc08fe4970, 0x849f0f75e5ce061b];
    let p = [
        0x40f75c3e90ebbf56,
        0x624e6e788fc9bc12,
        0x589431000df07902,
        0x9aef459e48a7ac73,
        0xa67dfc6202a89424,
        0x988adb52057842c0,
        0x7925243213523fe3,
        0xfef27a458a531028,
    ];
    let res1 = match double_scalar_mul_with_g_secp256k1(&k1, &k2, &p) {
        Some(point) => point,
        None => panic!("Double scalar multiplication with G failed"),
    };
    let res2 = match glv_double_scalar_mul_with_g_secp256k1(&k1, &k2, &p) {
        Some(point) => point,
        None => panic!("GLV double scalar multiplication with G failed"),
    };
    let res_exp = [
        0xd143c0e9ecf996a3,
        0xd974781400a006bf,
        0xf960eebd7c128a95,
        0x5e44767ac9426e5,
        0x53a5e16a27f9d9e6,
        0xbf5f20add02485f0,
        0x51a51aef05543d52,
        0x5afd5c6d73b4597d,
    ];
    assert_eq!(res1, res_exp);
    assert_eq!(res2, res_exp);

    let k1 = [0xf447e442a44c829e, 0xe979220cfe9824d3, 0x673913d78b5bdbfe, 0xd961172287f69999];
    let k2 = k1;
    let p = G_NEG;
    let res1 = double_scalar_mul_with_g_secp256k1(&k1, &k2, &p);
    let res2 = glv_double_scalar_mul_with_g_secp256k1(&k1, &k2, &p);
    assert_eq!(res1, None);
    assert_eq!(res2, None);
}
