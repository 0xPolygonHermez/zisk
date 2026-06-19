use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use super::FCALL_MSB_POS_384_ID;
    } else {
        use crate::zisklib::fcalls_impl::msb_pos_384::msb_pos_384;
    }
}

/// Returns `(limb, bit)` — the index of the highest-order limb and bit position of the
/// most significant set bit across two 384-bit values `x` and `y`.
///
/// Panics if both values are zero.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_msb_pos_384(
    x: &[u64; 6],
    y: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> (u64, u64) {
    #[cfg(not(zisk_guest))]
    {
        let (i, pos) = msb_pos_384(x, y);
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
        ziskos_fcall_param!(x, 8);
        ziskos_fcall_param!(y, 8);
        ziskos_fcall!(FCALL_MSB_POS_384_ID);
        (ziskos_fcall_get(), ziskos_fcall_get())
    }
}
