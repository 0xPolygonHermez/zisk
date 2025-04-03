use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(target_os = "ziskos")] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use crate::FCALL_MSB_POS_256_ID;
    }
}
#[allow(unused_variables)]
pub fn fcall_msb_pos_256(x: &[u64; 4], y: &[u64; 4]) -> (u64, u64) {
    #[cfg(not(target_os = "ziskos"))]
    unreachable!();
    #[cfg(target_os = "ziskos")]
    {
        ziskos_fcall_param!(x, 4);
        ziskos_fcall_param!(y, 4);
        ziskos_fcall!(FCALL_MSB_POS_256_ID);
        (ziskos_fcall_get(), ziskos_fcall_get())
    }
}
