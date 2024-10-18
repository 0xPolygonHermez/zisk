#[cfg(target_os = "ziskos")]
use core::arch::asm;

/// Executes the Keccak256 permutation on the given state.
///
/// ### Safety
///
/// The caller must ensure that `state` is valid pointer to data that is aligned along a four
/// byte boundary.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_keccak_f(state: *mut [u64; 25]) {
    #[cfg(target_os = "ziskos")]
    unsafe {
        asm!(
            "ecall",
            in("a7") crate::syscalls::KECCAKF,
            in("a0") state,
            in("a1") 0
        );
    }

    #[cfg(not(target_os = "ziskos"))]
    unreachable!()
}
