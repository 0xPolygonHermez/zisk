use tiny_keccak::{Hasher, Keccak};

use crate::{
    arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams},
    fcall_secp256k1_fn_inv, fcall_secp256k1_fp_sqrt,
    point256::SyscallPoint256,
    zisklib::lib::{
        secp256k1_fp_assert_nqr::secp256k1_fp_assert_nqr,
        secp256k1_msm::secp256k1_double_scalar_mul_with_g,
        utils::{gt, sub},
    },
};

/// Secp256k1 base field size
const P: [u64; 4] =
    [0xFFFFFFFEFFFFFC2F, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF];

/// Secp256k1 scalar field size
const N: [u64; 4] =
    [0xBFD25E8CD0364141, 0xBAAEDCE6AF48A03B, 0xFFFFFFFFFFFFFFFE, 0xFFFFFFFFFFFFFFFF];
const N_MINUS_ONE: [u64; 4] =
    [0xBFD25E8CD0364140, 0xBAAEDCE6AF48A03B, 0xFFFFFFFFFFFFFFFE, 0xFFFFFFFFFFFFFFFF];
const N_HALF: [u64; 4] =
    [0xDFE92F46681B20A0, 0x5D576E7357A4501D, 0xFFFFFFFFFFFFFFFF, 0x7FFFFFFFFFFFFFFF];

/// Given a hash `hash`, a recovery parity `v`, a signature (`r`, `s`), and a signature mode `mode`,
/// this function computes the address that signed the hash.
/// The `mode` indicates whether ecrecover is called as a precompiled contract (true) or as a transaction (false).
///
/// It also returns an error code:
/// - 0: No error
/// - 1: r should be greater than 0
/// - 2: r should be less than `N_MINUS_ONE`
/// - 3: s should be greater than 0
/// - 4: s should be less than `N_MINUS_ONE` or `N_HALF`
/// - 5: The recovery id should be either 0 or 1
/// - 6: No square root found for `y_sq`
/// - 7: The public key is the point at infinity
pub fn ecrecover(sig: &[u8; 65], msg: &[u8; 32], mode: bool) -> ([u8; 20], u8) {
    // Extract the signature components
    let mut r = [0u64; 4];
    let mut s = [0u64; 4];
    for i in 0..32 {
        r[3 - i / 8] |= (sig[i] as u64) << (8 * (7 - (i % 8)));
        s[3 - i / 8] |= (sig[32 + i] as u64) << (8 * (7 - (i % 8)));
    }

    // Check r is in the range [1, n-1]
    if r == [0, 0, 0, 0] {
        #[cfg(debug_assertions)]
        println!("r should be greater than 0");

        return ([0u8; 20], 1);
    } else if gt(&r, &N_MINUS_ONE) {
        #[cfg(debug_assertions)]
        println!("r should be less than N_MINUS_ONE: {:?}, but got {:?}", N_MINUS_ONE, r);

        return ([0u8; 20], 2);
    }

    // Check s is either in the range [1, n-1] (precompiled) or [1, (n-1)/2] (tx):
    let s_limit = if mode { N_MINUS_ONE } else { N_HALF };
    if s == [0, 0, 0, 0] {
        #[cfg(debug_assertions)]
        println!("s should be greater than 0");

        return ([0u8; 20], 3);
    } else if gt(&s, &s_limit) {
        #[cfg(debug_assertions)]
        println!("s should be less than s_limit: {:?}, but got {:?}", s_limit, s);

        return ([0u8; 20], 4);
    }

    // Extract the parity: 0 indicates that y is even, 1 indicates that y is odd
    let parity = sig[64] as u64;

    // Check the recovery id is a bit
    if parity > 1 {
        #[cfg(debug_assertions)]
        println!("parity should be either 0 or 1, but got {:?}", parity);

        return ([0u8; 20], 5);
    }

    // Get the hash
    let mut hash = [0u64; 4];
    for i in 0..32 {
        hash[3 - i / 8] |= (msg[i] as u64) << (8 * (7 - (i % 8)));
    }

    // In Ethereum, signatures where the x-coordinate of the resulting point is
    // greater than N are considered invalid. Hence, r = x as integers

    // Calculate the y-coordinate of the point: y = sqrt(x³ + 7)
    let mut params = SyscallArith256ModParams {
        a: &r,
        b: &r,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);
    let r_sq = *params.d;
    params.a = &r_sq;
    params.b = &r;
    params.c = &[7, 0, 0, 0];
    syscall_arith256_mod(&mut params);
    let y_sq = *params.d;

    // Hint the sqrt and verify it
    let y = match fcall_secp256k1_fp_sqrt(&y_sq, parity) {
        Some(y) => {
            // Check the received y is the sqrt
            params.a = &y;
            params.b = &y;
            params.c = &[0, 0, 0, 0];
            syscall_arith256_mod(&mut params);
            assert_eq!(*params.d, y_sq);
            y
        }
        None => {
            #[cfg(debug_assertions)]
            println!("No square root found for y_sq: {:?}", y_sq);

            // Check that y_sq is a non-quadratic residue
            secp256k1_fp_assert_nqr(&y_sq);

            return ([0u8; 20], 6);
        }
    };

    // Check the received parity of the y-coordinate is correct, otherwise MAP
    let y_parity = y[0] & 1;
    assert_eq!(y_parity, parity);

    // Calculate the public key

    // Hint the inverse and verify it
    let r_inv = fcall_secp256k1_fn_inv(&r);
    let mut params = SyscallArith256ModParams {
        a: &r,
        b: &r_inv,
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);
    assert_eq!(*params.d, [0x1, 0x0, 0x0, 0x0]);

    // Compute k1 = (-hash * r_inv) % N
    params.a = &hash;
    params.b = &r_inv;
    params.c = &[0, 0, 0, 0];
    syscall_arith256_mod(&mut params);
    let k1 = sub(&N, params.d);

    // Compute k2 = (s * r_inv) % N
    params.a = &s;
    params.b = &r_inv;
    syscall_arith256_mod(&mut params);
    let k2 = params.d;

    // Calculate the public key
    let p = SyscallPoint256 { x: r, y };
    let (pk_is_infinity, pk) = secp256k1_double_scalar_mul_with_g(&k1, k2, &p);
    if pk_is_infinity {
        #[cfg(debug_assertions)]
        println!("The public key is the point at infinity");

        return ([0u8; 20], 7);
    }

    // Compute the hash of the public key
    // Q: Is it better to use a hash API that accepts u64 instead of u8?
    // Q: Substitute the function by low-level stuff!
    let mut buf = [0u8; 64];
    for i in 0..4 {
        buf[i * 8..(i + 1) * 8].copy_from_slice(&pk.x[3 - i].to_be_bytes());
        buf[32 + i * 8..32 + (i + 1) * 8].copy_from_slice(&pk.y[3 - i].to_be_bytes());
    }

    let mut pk_hash = [0u8; 32];
    let mut keccak = Keccak::v256();
    keccak.update(&buf);
    keccak.finalize(&mut pk_hash);

    // Return the right-most 20 bytes of the hash
    let mut addr = [0u8; 20];
    for i in 0..20 {
        addr[i] = pk_hash[12 + i];
    }
    (addr, 0)
}
