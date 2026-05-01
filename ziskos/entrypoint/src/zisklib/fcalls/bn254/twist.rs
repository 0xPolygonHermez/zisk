use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{
            ziskos_fcall, ziskos_fcall_param,
            zisklib::{FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID, FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID}
        };
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use crate::zisklib::fcalls_impl::bn254::{bn254_twist_add_line_coeffs, bn254_twist_dbl_line_coeffs};
    }

}

/// Computes the coefficients for the line defining the addition of two points on the BN254 twist.
///
/// `fcall_bn254_twist_add_line_coeffs` takes two points on the twist, each represented as an array of 16 `u64` values
/// (8 for x and 8 for y), and returns the line coefficients as two arrays of 8 `u64` values each (lambda and mu).
///
/// ### Safety
///
/// The caller must ensure that data is aligned to a 64-bit boundary.
///
/// The caller must ensure that x-coordinates of the input points are distinct.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bn254_twist_add_line_coeffs(
    p1: &[u64; 16],
    p2: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 8], [u64; 8]) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let x1: [u64; 8] = p1[0..8].try_into().unwrap();
        let y1: [u64; 8] = p1[8..16].try_into().unwrap();
        let x2: [u64; 8] = p2[0..8].try_into().unwrap();
        let y2: [u64; 8] = p2[8..16].try_into().unwrap();
        let (lambda, mu): ([u64; 8], [u64; 8]) = bn254_twist_add_line_coeffs(&x1, &y1, &x2, &y2);
        #[cfg(feature = "hints")]
        {
            hints.push(16);
            hints.extend_from_slice(&lambda);
            hints.extend_from_slice(&mu);
        }
        (lambda, mu)
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(p1, 16);
        ziskos_fcall_param!(p2, 16);
        ziskos_fcall!(FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID);
        #[cfg(not(feature = "inputcpy"))]
        {
            (
                [
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                ],
                [
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                ],
            )
        }
        #[cfg(feature = "inputcpy")]
        {
            use core::mem::MaybeUninit;
            let mut lambda: MaybeUninit<[u64; 8]> = MaybeUninit::uninit();
            ziskos_inputcpy!(lambda, 8 * 8);
            let mut mu: MaybeUninit<[u64; 8]> = MaybeUninit::uninit();
            ziskos_inputcpy!(mu, 8 * 8);
            unsafe { (lambda.assume_init(), mu.assume_init()) }
        }
    }
}

/// Computes the coefficients for the line defining the doubling of a point on the BN254 twist.
///
/// `fcall_bn254_twist_dbl_line_coeffs` takes a point on the twist, represented as an array of 16 `u64` values
/// (8 for x and 8 for y), and returns the line coefficients as two arrays of 8 `u64` values each (lambda and mu).
///
/// ### Safety
///
/// The caller must ensure that data is aligned to a 64-bit boundary.
///
/// The caller must ensure that the y-coordinate of the input point is non-zero.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bn254_twist_dbl_line_coeffs(
    p: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 8], [u64; 8]) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let x1: [u64; 8] = p[0..8].try_into().unwrap();
        let y1: [u64; 8] = p[8..16].try_into().unwrap();
        let (lambda, mu): ([u64; 8], [u64; 8]) = bn254_twist_dbl_line_coeffs(&x1, &y1);
        #[cfg(feature = "hints")]
        {
            hints.push(16);
            hints.extend_from_slice(&lambda);
            hints.extend_from_slice(&mu);
        }
        (lambda, mu)
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(p, 16);
        ziskos_fcall!(FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID);
        #[cfg(not(feature = "inputcpy"))]
        {
            (
                [
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                ],
                [
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                    ziskos_fcall_get(),
                ],
            )
        }
        #[cfg(feature = "inputcpy")]
        {
            use core::mem::MaybeUninit;
            let mut lambda: MaybeUninit<[u64; 8]> = MaybeUninit::uninit();
            ziskos_inputcpy!(lambda, 8 * 8);
            let mut mu: MaybeUninit<[u64; 8]> = MaybeUninit::uninit();
            ziskos_inputcpy!(mu, 8 * 8);
            unsafe { (lambda.assume_init(), mu.assume_init()) }
        }
    }
}
