use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use super::FCALL_INPUT_READY_ID;
    }
}

/// Signals the host that the guest is ready to consume input at `address`.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the correctness
/// of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_input_ready(address: &u64) {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        // TODO: wait for input to be ready at the given address, then check the input length vs. address and return an error if the input is not long enough. For now, we just return immediately.
        unimplemented!("fcall_input_ready is not implemented for non-zisk targets");
    }
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        ziskos_fcall_param!(*address, 1); // Number of inputs
        ziskos_fcall!(FCALL_INPUT_READY_ID);
    }
}
