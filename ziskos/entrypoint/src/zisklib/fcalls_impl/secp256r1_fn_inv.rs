// TODO!!!!! IN LINUUUUX

// cfg_if::cfg_if! {
//     if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
//         use lib_c::secp256r1_fn_inv_c;

//         pub fn fcall_secp256r1_fn_inv(params: &[u64], results: &mut [u64]) -> i64 {
//             // Perform the inversion
//             let res_c_call = secp256r1_fn_inv_c(params, results);
//             if res_c_call == 0 {
//                 4
//             } else {
//                 res_c_call as i64
//             }
//         }
//     } else {
use lazy_static::lazy_static;
use num_bigint::BigUint;

use super::utils::{from_limbs_le, to_limbs_le};

lazy_static! {
    pub static ref N: BigUint = BigUint::parse_bytes(
        b"ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551",
        16
    )
    .unwrap();
}

pub fn fcall_secp256r1_fn_inv(params: &[u64], results: &mut [u64]) -> i64 {
    // Get the input
    let a: &[u64; 4] = &params[0..4].try_into().unwrap();

    // Perform the inversion using fn inversion
    let inv = secp256r1_fn_inv(a);

    // Store the result
    results[0..4].copy_from_slice(&inv);

    4
}

fn secp256r1_fn_inv(a: &[u64; 4]) -> [u64; 4] {
    let a_big = from_limbs_le(a);
    let inv = a_big.modinv(&N);
    match inv {
        Some(inverse) => to_limbs_le(&inverse),
        None => panic!("Inverse does not exist"),
    }
}
// }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv_one() {
        let x = [1, 0, 0, 0];
        let expected_inv = [1, 0, 0, 0];

        let mut results = [0; 4];
        fcall_secp256r1_fn_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }

    #[test]
    fn test_inv() {
        let x = [0xf9ee4256a589409f, 0xa21a3985f17502d0, 0xb3eb393d00dc480c, 0x142def02c537eced];
        let expected_inv =
            [0x7450938531a554a4, 0x49a5e61e420cf950, 0x5e5e8115e302f1dd, 0xe4bac2152faee1f6];

        let mut results = [0; 4];
        fcall_secp256r1_fn_inv(&x, &mut results);
        assert_eq!(results, expected_inv);
    }
}
