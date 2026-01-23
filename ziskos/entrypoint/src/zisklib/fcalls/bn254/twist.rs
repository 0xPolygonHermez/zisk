use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{
            ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param,
            zisklib::{FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID, FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID}
        };
        } else {
        use crate::zisklib::fcalls_impl::bn254::{bn254_twist_add_line_coeffs, bn254_twist_dbl_line_coeffs};
    }

}

/// Computes the coefficients for the line defining the addition of two points on the `bn254` twist.
///
/// ### Safety
///
/// The caller must ensure that the input pointer (`p_value`) is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bn254_twist_add_line_coeffs(
    p1_value: &[u64; 16],
    p2_value: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 8], [u64; 8]) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let x1: [u64; 8] = p1_value[0..8].try_into().unwrap();
        let y1: [u64; 8] = p1_value[8..16].try_into().unwrap();
        let x2: [u64; 8] = p2_value[0..8].try_into().unwrap();
        let y2: [u64; 8] = p2_value[8..16].try_into().unwrap();
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
        ziskos_fcall_param!(p1_value, 16);
        ziskos_fcall_param!(p2_value, 16);
        ziskos_fcall!(FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID);
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
}

/// Computes the coefficients for the line defining the doubling of a point on the `bn254` twist.
///
/// ### Safety
///
/// The caller must ensure that the input pointer (`p_value`) is valid and aligned to an 8-byte boundary.
///
/// Note that this is a *free-input call*, meaning the Zisk VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_bn254_twist_dbl_line_coeffs(
    p_value: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 8], [u64; 8]) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let x1: [u64; 8] = p_value[0..8].try_into().unwrap();
        let y1: [u64; 8] = p_value[8..16].try_into().unwrap();
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
        ziskos_fcall_param!(p_value, 16);
        ziskos_fcall!(FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID);
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
}
