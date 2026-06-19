use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{
            ziskos_fcall, ziskos_fcall_param,
            zisklib::{FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS_ID, FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS_ID},
        };
        #[cfg(not(feature = "inputcpy"))]
        use crate::ziskos_fcall_get;
        #[cfg(feature = "inputcpy")]
        use crate::ziskos_inputcpy;
    } else {
        use crate::zisklib::fcalls_impl::bls12_381::{bls12_381_twist_add_line_coeffs, bls12_381_twist_dbl_line_coeffs};
    }
}

/// Computes the coefficients for the line defining the addition of two points on the BLS12-381 twist.
///
/// `fcall_bls12_381_twist_add_line_coeffs` takes two points on the twist, each represented as an array of 24 `u64` values
/// (12 for x and 12 for y), and returns the line coefficients as two arrays of 12 `u64` values each (lambda and mu).
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
pub fn fcall_bls12_381_twist_add_line_coeffs(
    p1: &[u64; 24],
    p2: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 12], [u64; 12]) {
    #[cfg(not(zisk_guest))]
    {
        let x1: [u64; 12] = p1[0..12].try_into().unwrap();
        let y1: [u64; 12] = p1[12..24].try_into().unwrap();
        let x2: [u64; 12] = p2[0..12].try_into().unwrap();
        let y2: [u64; 12] = p2[12..24].try_into().unwrap();
        let (lambda, mu): ([u64; 12], [u64; 12]) =
            bls12_381_twist_add_line_coeffs(&x1, &y1, &x2, &y2);
        #[cfg(feature = "hints")]
        {
            hints.push(24);
            hints.extend_from_slice(&lambda);
            hints.extend_from_slice(&mu);
        }

        (lambda, mu)
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(p1, 24);
        ziskos_fcall_param!(p2, 24);
        ziskos_fcall!(FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS_ID);
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
            let mut lambda: MaybeUninit<[u64; 12]> = MaybeUninit::uninit();
            ziskos_inputcpy!(lambda, 12 * 8);
            let mut mu: MaybeUninit<[u64; 12]> = MaybeUninit::uninit();
            ziskos_inputcpy!(mu, 12 * 8);
            (unsafe { lambda.assume_init() }, unsafe { mu.assume_init() })
        }
    }
}

/// Computes the coefficients for the line defining the doubling of a point on the BLS12-381 twist.
///
/// `fcall_bls12_381_twist_dbl_line_coeffs` takes a point on the twist, represented as an array of 24 `u64` values
/// (12 for x and 12 for y), and returns the line coefficients as two arrays of 12 `u64` values each (lambda and mu).
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
pub fn fcall_bls12_381_twist_dbl_line_coeffs(
    p: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 12], [u64; 12]) {
    #[cfg(not(zisk_guest))]
    {
        let x: [u64; 12] = p[0..12].try_into().unwrap();
        let y: [u64; 12] = p[12..24].try_into().unwrap();
        let (lambda, mu): ([u64; 12], [u64; 12]) = bls12_381_twist_dbl_line_coeffs(&x, &y);
        #[cfg(feature = "hints")]
        {
            hints.push(24);
            hints.extend_from_slice(&lambda);
            hints.extend_from_slice(&mu);
        }
        (lambda, mu)
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(p, 24);
        ziskos_fcall!(FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS_ID);
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
            let mut lambda: MaybeUninit<[u64; 12]> = MaybeUninit::uninit();
            ziskos_inputcpy!(lambda, 12 * 8);
            let mut mu: MaybeUninit<[u64; 12]> = MaybeUninit::uninit();
            ziskos_inputcpy!(mu, 12 * 8);
            (unsafe { lambda.assume_init() }, unsafe { mu.assume_init() })
        }
    }
}
