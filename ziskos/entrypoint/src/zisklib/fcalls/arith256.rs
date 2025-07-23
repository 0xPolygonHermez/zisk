use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param, FCALL_ARITH256_DIV_REM_ID};
    }
}

#[allow(unused_variables)]
pub fn fcall_div_rem_256(p1_value: &[u64; 4], p2_value: &[u64; 4]) -> ([u64; 4], [u64; 4]) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    unreachable!();
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(p1_value, 4);
        ziskos_fcall_param!(p2_value, 4);
        ziskos_fcall!(FCALL_ARITH256_DIV_REM_ID);
        (
            [ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get()],
            [ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get(), ziskos_fcall_get()],
        )
    }
}
