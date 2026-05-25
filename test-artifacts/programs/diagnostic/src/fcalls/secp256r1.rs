use ziskos::zisklib::fcall_secp256r1_fn_inv;

pub fn diagnostic_secp256r1() {
    diagnostic_secp256r1_fn_inv();
}

fn diagnostic_secp256r1_fn_inv() {
    let x = [1, 0, 0, 0];
    let expected_inv = [1, 0, 0, 0];
    let inv = fcall_secp256r1_fn_inv(&x);
    assert_eq!(inv, expected_inv);

    let x = [0xf9ee4256a589409f, 0xa21a3985f17502d0, 0xb3eb393d00dc480c, 0x142def02c537eced];
    let expected_inv =
        [0x7450938531a554a4, 0x49a5e61e420cf950, 0x5e5e8115e302f1dd, 0xe4bac2152faee1f6];
    let inv = fcall_secp256r1_fn_inv(&x);
    assert_eq!(inv, expected_inv);
}
