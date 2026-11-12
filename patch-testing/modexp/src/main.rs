//! zisk-patch-modexp / aurora-engine-modexp: big-integer modexp routed to
//! `modexp_u64_c`. Cross-checked against num-bigint's `modpow`.

#![no_main]
ziskos::entrypoint!(main);

use aurora_engine_modexp::modexp;
use num_bigint::BigUint;
use patch_test_common::{hex, Rng, Suite};

fn strip(v: &[u8]) -> Vec<u8> {
    let i = v.iter().position(|&b| b != 0).unwrap_or(v.len());
    v[i..].to_vec()
}

fn reference(base: &[u8], exp: &[u8], m: &[u8]) -> Vec<u8> {
    let m = BigUint::from_bytes_be(m);
    if m == BigUint::from(0u8) {
        return Vec::new();
    }
    BigUint::from_bytes_be(base).modpow(&BigUint::from_bytes_be(exp), &m).to_bytes_be()
}

fn check(t: &mut Suite, what: &str, base: &[u8], exp: &[u8], m: &[u8]) {
    t.eq(what, strip(&modexp(base, exp, m)), strip(&reference(base, exp, m)));
}

fn main() {
    let mut t = Suite::new("modexp");

    // EIP-198 example: 3^(p-1) mod p == 1 for the secp256k1 field prime.
    let p = hex("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f");
    let p_minus_1 = hex("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2e");
    t.eq("EIP-198: 3^(p-1) mod p", strip(&modexp(&[3], &p_minus_1, &p)), vec![1]);
    t.eq("2^10 mod 1000", strip(&modexp(&[2], &[10], &hex("03e8"))), vec![24]);

    // Edge cases.
    t.eq("modulus 0 -> empty", modexp(&[5], &[3], &[0]), Vec::<u8>::new());
    t.eq("modulus 1 -> 0", strip(&modexp(&[5], &[3], &[1])), Vec::<u8>::new());
    t.eq("exp 0 -> 1", strip(&modexp(&[5], &[0], &[7])), vec![1]);
    t.eq("empty exp -> 1", strip(&modexp(&[5], &[], &[7])), vec![1]);
    t.eq("base 0 -> 0", strip(&modexp(&[0], &[3], &[7])), Vec::<u8>::new());
    check(&mut t, "base > modulus", &hex("ffffffffffffffffffffffffffffffffff"), &[65, 1], &hex("0100000000000000000001"));
    check(&mut t, "leading zero bytes", &hex("0000000003"), &hex("000005"), &hex("00000000000007"));

    // Random sizes, including the byte lengths that cross 64-bit limb boundaries
    // and an RSA-2048-sized modulus.
    let mut rng = Rng::new(0x30de);
    for (bl, el, ml) in [
        (1, 1, 1),
        (8, 8, 8),
        (9, 3, 9),
        (31, 32, 32),
        (32, 32, 32),
        (33, 5, 33),
        (64, 64, 64),
        (100, 17, 97),
        (256, 3, 256),
        (256, 256, 256),
        (17, 300, 128),
    ] {
        for i in 0..3 {
            let (b, e, mut m) = (rng.bytes(bl), rng.bytes(el), rng.bytes(ml));
            if i == 0 {
                // Even and odd moduli take different paths in most implementations.
                *m.last_mut().unwrap() |= 1;
            } else if i == 1 {
                *m.last_mut().unwrap() &= !1;
            }
            check(&mut t, &format!("random b{bl} e{el} m{ml} #{i}"), &b, &e, &m);
        }
    }

    t.finish();
}
