//! zisk-patch-ruint: 256-bit modular arithmetic routed to the zisklib
//! (`zkvm_mulmod256` / `zkvm_add_mod256`).
//!
//! Every U256 op is cross-checked against the same computation done in U512, which is not
//! accelerated (the patches only take over the `BITS == 256` case).

#![no_main]
ziskos::entrypoint!(main);

use patch_test_common::{Rng, Suite};
use ruint::aliases::{U256, U512};

fn wide(x: U256) -> U512 {
    U512::from(x)
}

fn narrow(x: U512) -> U256 {
    x.to::<U256>()
}

fn low256(x: U512) -> U256 {
    let l = x.as_limbs();
    U256::from_limbs([l[0], l[1], l[2], l[3]])
}

fn rand_u256(rng: &mut Rng) -> U256 {
    // Mix full-width values with shorter ones so that small moduli and
    // high-limb-zero paths get exercised too.
    let bits = match rng.next_u64() % 4 {
        0 => 256,
        1 => 192,
        2 => 128,
        _ => 1 + (rng.next_u64() % 256) as usize,
    };
    let v = U256::from_limbs([rng.next_u64(), rng.next_u64(), rng.next_u64(), rng.next_u64()]);
    if bits == 256 {
        v
    } else {
        v & ((U256::from(1u8) << bits) - U256::from(1u8))
    }
}

fn check_pair(t: &mut Suite, a: U256, b: U256, m: U256) {
    let ctx = format!("a={a:#x} b={b:#x} m={m:#x}");
    if m.is_zero() {
        t.eq(&format!("mul_mod m=0 {ctx}"), a.mul_mod(b, m), U256::ZERO);
        t.eq(&format!("add_mod m=0 {ctx}"), a.add_mod(b, m), U256::ZERO);
    } else {
        t.eq(&format!("mul_mod {ctx}"), a.mul_mod(b, m), narrow(wide(a) * wide(b) % wide(m)));
        t.eq(&format!("add_mod {ctx}"), a.add_mod(b, m), narrow((wide(a) + wide(b)) % wide(m)));
        t.eq(&format!("reduce_mod {ctx}"), a.reduce_mod(m), narrow(wide(a) % wide(m)));
        let (q, r) = a.div_rem(m);
        let (wq, wr) = wide(a).div_rem(wide(m));
        t.eq(&format!("div_rem {ctx}"), (q, r), (narrow(wq), narrow(wr)));
    }
    let full = wide(a) * wide(b);
    t.eq(&format!("overflowing_mul {ctx}"), a.overflowing_mul(b), (low256(full), full >> 256 != U512::ZERO));
    t.eq(&format!("wrapping_mul {ctx}"), a.wrapping_mul(b), low256(full));
}

fn check_pow(t: &mut Suite, a: U256, e: u64) {
    let mut acc = U512::from(1u8);
    let mut base = wide(a);
    let mut k = e;
    while k > 0 {
        if k & 1 == 1 {
            acc = wide(low256(acc * base));
        }
        base = wide(low256(base * base));
        k >>= 1;
    }
    t.eq(&format!("wrapping_pow a={a:#x} e={e}"), a.wrapping_pow(U256::from(e)), low256(acc));
}

fn main() {
    let mut t = Suite::new(env!("CARGO_PKG_NAME"));

    let p = U256::from_str_radix("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f", 16).unwrap();
    let n = U256::from_str_radix("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001", 16).unwrap();
    let edge = [U256::ZERO, U256::from(1u8), U256::from(2u8), p - U256::from(1u8), p, n, U256::MAX - U256::from(1u8), U256::MAX];
    for &a in &edge {
        for &b in &edge {
            for &m in &[U256::ZERO, U256::from(1u8), U256::from(7u8), p, n, U256::MAX] {
                check_pair(&mut t, a, b, m);
            }
        }
    }

    let mut rng = Rng::new(0x2c1a7);
    for _ in 0..300 {
        let (a, b, m) = (rand_u256(&mut rng), rand_u256(&mut rng), rand_u256(&mut rng));
        check_pair(&mut t, a, b, m);
    }

    for e in [0u64, 1, 2, 3, 17, 255, 256, 65537, u64::MAX] {
        check_pow(&mut t, U256::from(3u8), e);
        check_pow(&mut t, p - U256::from(1u8), e);
        let a = rand_u256(&mut rng);
        check_pow(&mut t, a, e);
    }

    // Known answer: 3^(p-1) mod p == 1 (Fermat), computed through mul_mod.
    let mut acc = U256::from(1u8);
    let mut base = U256::from(3u8);
    let mut e = p - U256::from(1u8);
    while !e.is_zero() {
        if e.bit(0) {
            acc = acc.mul_mod(base, p);
        }
        base = base.mul_mod(base, p);
        e >>= 1;
    }
    t.eq("3^(p-1) mod p via mul_mod", acc, U256::from(1u8));

    t.finish();
}
