use ziskos::zisklib::ecdsa_verify_secp256k1;

pub fn ecdsa_tests() {
    // Verify (valids)
    let pk = [
        0x3bcfdc2aca47e0f2,
        0xa739d5cc6b89e9b5,
        0x35b73cc431afc6bc,
        0xe1ea4273f638d4ae,
        0xc6402318ee33448e,
        0x9f18c242b8df8bb6,
        0x934a8dfdd797e1c4,
        0x3840aa9c4d86557e,
    ];
    let z = [0x1bf86a1816a52f52, 0xd31e26c3da73dda8, 0xa3b71997594da038, 0x17560495f6944673];
    let r = [0x68df7d8d7e0fb36b, 0xc2189fe681cd6e78, 0xc85ba1fd6238ecb5, 0x3e125456c8338994];
    let s = [0xd4e89d1ae75aeea2, 0xb8e33178783bd1a3, 0x866acebc9e141ec, 0x3a816b1c33739e41];
    let res = ecdsa_verify_secp256k1(&pk, &z, &r, &s);
    assert!(res);

    let pk = [
        0x563cc8bd265493e8,
        0xe26ca51d73493bb1,
        0xbaf413f523859dc3,
        0x8977dd29be12980c,
        0x576f2adf398ee11b,
        0xe5cf41decfb89b8b,
        0x520997ce18f48f65,
        0x5ca08a9cbf10b944,
    ];
    let z = [0xab313c87e8099e4d, 0x98e191bbf16e2538, 0x5fad2b6370bed1a0, 0xbda0ec6db2b2f290];
    let r = [0x320ca78102f69537, 0x548ce6ea0e9f500c, 0xac0d2ea7c5b22534, 0x560ef25130dd8a78];
    let s = [0xdfd826f31fb981d0, 0x48fed994c3e102e5, 0xc2ffab95e65eef0f, 0x18b94747713e154f];
    let res = ecdsa_verify_secp256k1(&pk, &z, &r, &s);
    assert!(res);
}
