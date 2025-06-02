use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use crate::{FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID, FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID};
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
pub fn fcall_bn254_add_line_coeffs(
    p1_value: &[u64; 16],
    p2_value: &[u64; 16],
) -> ([u64; 8], [u64; 8]) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!();
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
pub fn fcall_bn254_dbl_line_coeffs(p_value: &[u64; 16]) -> ([u64; 8], [u64; 8]) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!();
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
