use ziskos::zisklib::{neg_fn_secp256k1, reduce_fn_secp256k1, sub_fn_secp256k1};

/*
sage: p = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F
sage: n = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
sage: F = GF(p)
sage: Fn = GF(n)
sage: E = EllipticCurve(F, [0,7])
sage: G = E(0x79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798, 0x483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8)
*/
pub fn scalar_tests() {
    // Reduction
    let a = [0xc20e6605244aad3e, 0xa62f267df99427c6, 0xffffffffffffffff, 0xffffffffffffffff];
    let res = reduce_fn_secp256k1(&a);
    let res_exp = [0x23c077854146bfd, 0xeb8049974a4b878b, 0x0, 0x0];
    assert_eq!(res, res_exp);

    // Negation
    let a = [0; 4];
    let res = neg_fn_secp256k1(&a);
    let res_exp = a;
    assert_eq!(res, res_exp);

    let a = [0x334d5469d32c3b5b, 0x2b7465755356f643, 0x60e777bde950c3b6, 0x3db52491030af31e];
    let res = neg_fn_secp256k1(&a);
    let res_exp = [0x8c850a22fd0a05e6, 0x8f3a77715bf1a9f8, 0x9f18884216af3c48, 0xc24adb6efcf50ce1];
    assert_eq!(res, res_exp);

    // Subtraction
    let a = [0x334d5469d32c3b5b, 0x2b7465755356f643, 0x60e777bde950c3b6, 0x3db52491030af31e];
    let b = [0x7187f6e16ae0e273, 0xb1428809d3b91b1b, 0x4abaebb300997779, 0xc95bd1038d960573];
    let res = sub_fn_secp256k1(&a, &b);
    let res_exp = [0x8197bc1538819a29, 0x34e0ba522ee67b63, 0x162c8c0ae8b74c3b, 0x7459538d7574edab];
    assert_eq!(res, res_exp);
}
