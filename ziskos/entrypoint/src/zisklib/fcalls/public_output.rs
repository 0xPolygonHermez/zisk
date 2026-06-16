use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_param};
        use super::FCALL_PUBLIC_OUTPUT_ID;
    }
}

/// Streams `len` bytes of public output located at `ptr` to the host.
///
/// This is a *free-input call* used purely for its host-side side effect: the host appends the
/// bytes to its public-output buffer, so the proof can carry the plaintext public output
/// alongside the SHA-256 digest committed in `OUTPUT_ADDR`. It is **unconstrained** — soundness
/// rests entirely on that digest, and the verifier rehashes the carried bytes to bind them. The
/// VM does not verify anything about this call, and the guest reads no result back.
///
/// Called once per `write_output` chunk so an unbounded amount of output can be streamed without
/// buffering it in guest memory.
#[allow(unused_variables)]
pub fn fcall_public_output(ptr: *const u8, len: usize) {
    #[cfg(zisk_guest)]
    {
        // Pass (ptr, len) as two direct-value params; the host reads `len` bytes at `ptr`.
        ziskos_fcall_param!(ptr as u64, 1);
        ziskos_fcall_param!(len as u64, 1);
        ziskos_fcall!(FCALL_PUBLIC_OUTPUT_ID);
    }
    #[cfg(not(zisk_guest))]
    {
        // On native targets the public output is captured by the host execution environment
        // (the emulator's fcall handler) rather than here; this is a no-op.
        let _ = (ptr, len);
    }
}
