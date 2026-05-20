use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use super::FCALL_MSB_POS_256_ID;
    } else {
        use crate::zisklib::fcalls_impl::msb_pos_256::msb_pos_256;
    }
}

/// Returns `(limb, bit)` — the index of the highest-order limb and bit position of the
/// most significant set bit of the 256-bit value `x`.
///
/// Panics if the value is zero.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_msb_pos_256(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> (u64, u64) {
    #[cfg(not(zisk_guest))]
    {
        let (i, pos) = msb_pos_256(x, 1);
        #[cfg(feature = "hints")]
        {
            hints.push(2);
            hints.push(i as u64);
            hints.push(pos as u64);
        }
        (i as u64, pos as u64)
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(1, 1); // Number of inputs
        ziskos_fcall_param!(x, 4);
        ziskos_fcall!(FCALL_MSB_POS_256_ID);
        (ziskos_fcall_get(), ziskos_fcall_get())
    }
}

/// Returns `(limb, bit)` — the index of the highest-order limb and bit position of the
/// most significant set bit across two 256-bit values `x` and `y`.
///
/// Panics if both values are zero.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_msb_pos_256_2(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> (u64, u64) {
    #[cfg(not(zisk_guest))]
    {
        let tmp: [u64; 8] = [x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3]];
        let (i, pos) = msb_pos_256(&tmp, 2);
        #[cfg(feature = "hints")]
        {
            hints.push(2);
            hints.push(i as u64);
            hints.push(pos as u64);
        }
        (i as u64, pos as u64)
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(2, 1); // Number of inputs
        ziskos_fcall_param!(x, 4);
        ziskos_fcall_param!(y, 4);
        ziskos_fcall!(FCALL_MSB_POS_256_ID);
        (ziskos_fcall_get(), ziskos_fcall_get())
    }
}

/// Returns `(limb, bit)` — the index of the highest-order limb and bit position of the
/// most significant set bit across three 256-bit values `x`, `y`, and `z`.
///
/// Panics if all values are zero.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_msb_pos_256_3(
    x: &[u64; 4],
    y: &[u64; 4],
    z: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> (u64, u64) {
    #[cfg(not(zisk_guest))]
    {
        let tmp: [u64; 12] =
            [x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3], z[0], z[1], z[2], z[3]];
        let (i, pos) = msb_pos_256(&tmp, 3);
        #[cfg(feature = "hints")]
        {
            hints.push(2);
            hints.push(i as u64);
            hints.push(pos as u64);
        }
        (i as u64, pos as u64)
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(3, 1); // Number of inputs
        ziskos_fcall_param!(x, 4);
        ziskos_fcall_param!(y, 4);
        ziskos_fcall_param!(z, 4);
        ziskos_fcall!(FCALL_MSB_POS_256_ID);
        (ziskos_fcall_get(), ziskos_fcall_get())
    }
}
