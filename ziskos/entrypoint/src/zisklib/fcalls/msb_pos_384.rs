use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use crate::FCALL_MSB_POS_384_ID;
    }
}
#[allow(unused_variables)]
pub fn fcall_msb_pos_384(x: &[u64; 6], y: &[u64; 6]) -> (u64, u64) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!();
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(x, 8);
        ziskos_fcall_param!(y, 8);
        ziskos_fcall!(FCALL_MSB_POS_384_ID);
        (ziskos_fcall_get(), ziskos_fcall_get())
    }
}
