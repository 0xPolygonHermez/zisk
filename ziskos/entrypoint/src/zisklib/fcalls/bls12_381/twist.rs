use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{
            ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param,
            zisklib::{FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS_ID, FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS_ID},
        };
    } else {
        use crate::zisklib::fcalls_impl::bls12_381::{bls12_381_twist_add_line_coeffs, bls12_381_twist_dbl_line_coeffs};
    }
}

/// Computes the coefficients for the line defining the addition of two points on the `bls12_381` twist.
///
/// ### Safety
///
/// The caller must ensure that the input pointer (`p_value`) is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bls12_381_twist_add_line_coeffs(
    p1_value: &[u64; 24],
    p2_value: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 12], [u64; 12]) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let x1: [u64; 12] = p1_value[0..12].try_into().unwrap();
        let y1: [u64; 12] = p1_value[12..24].try_into().unwrap();
        let x2: [u64; 12] = p2_value[0..12].try_into().unwrap();
        let y2: [u64; 12] = p2_value[12..24].try_into().unwrap();
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
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(p1_value, 24);
        ziskos_fcall_param!(p2_value, 24);
        ziskos_fcall!(FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS_ID);
        (
            [
                ziskos_fcall_get(), // 0
                ziskos_fcall_get(), // 1
                ziskos_fcall_get(), // 2
                ziskos_fcall_get(), // 3
                ziskos_fcall_get(), // 4
                ziskos_fcall_get(), // 5
                ziskos_fcall_get(), // 6
                ziskos_fcall_get(), // 7
                ziskos_fcall_get(), // 8
                ziskos_fcall_get(), // 9
                ziskos_fcall_get(), // 10
                ziskos_fcall_get(), // 11
            ],
            [
                ziskos_fcall_get(), // 0
                ziskos_fcall_get(), // 1
                ziskos_fcall_get(), // 2
                ziskos_fcall_get(), // 3
                ziskos_fcall_get(), // 4
                ziskos_fcall_get(), // 5
                ziskos_fcall_get(), // 6
                ziskos_fcall_get(), // 7
                ziskos_fcall_get(), // 8
                ziskos_fcall_get(), // 9
                ziskos_fcall_get(), // 10
                ziskos_fcall_get(), // 11
            ],
        )
    }
}

/// Computes the coefficients for the line defining the doubling of a point on the `bls12_381` twist.
///
/// ### Safety
///
/// The caller must ensure that the input pointer (`p_value`) is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bls12_381_twist_dbl_line_coeffs(
    p_value: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 12], [u64; 12]) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let x: [u64; 12] = p_value[0..12].try_into().unwrap();
        let y: [u64; 12] = p_value[12..24].try_into().unwrap();
        let (lambda, mu): ([u64; 12], [u64; 12]) = bls12_381_twist_dbl_line_coeffs(&x, &y);
        #[cfg(feature = "hints")]
        {
            hints.push(24);
            hints.extend_from_slice(&lambda);
            hints.extend_from_slice(&mu);
        }
        (lambda, mu)
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(p_value, 24);
        ziskos_fcall!(FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS_ID);
        (
            [
                ziskos_fcall_get(), // 0
                ziskos_fcall_get(), // 1
                ziskos_fcall_get(), // 2
                ziskos_fcall_get(), // 3
                ziskos_fcall_get(), // 4
                ziskos_fcall_get(), // 5
                ziskos_fcall_get(), // 6
                ziskos_fcall_get(), // 7
                ziskos_fcall_get(), // 8
                ziskos_fcall_get(), // 9
                ziskos_fcall_get(), // 10
                ziskos_fcall_get(), // 11
            ],
            [
                ziskos_fcall_get(), // 0
                ziskos_fcall_get(), // 1
                ziskos_fcall_get(), // 2
                ziskos_fcall_get(), // 3
                ziskos_fcall_get(), // 4
                ziskos_fcall_get(), // 5
                ziskos_fcall_get(), // 6
                ziskos_fcall_get(), // 7
                ziskos_fcall_get(), // 8
                ziskos_fcall_get(), // 9
                ziskos_fcall_get(), // 10
                ziskos_fcall_get(), // 11
            ],
        )
    }
}
