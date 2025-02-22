//! Keccak system call interception

#[cfg(target_os = "ziskos")]
use core::arch::asm;

/// Executes the Keccak256 permutation on the given state.
///
/// The keccak system call writes the KECCAKF constant to the A7 register, the address of the
/// input/output memory buffer to the A0 register, and a 0 to the A1 register.
/// The Zisk
/// The Keccak-f code will get the input state data (1600 bits = 200 bytes) from that address, hash
/// the bits, and write the output state data (same size) to the same address as a result.
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
            in("a1") 0,
            out("ra") _,
        );
    }
    #[cfg(not(target_os = "ziskos"))]
    unreachable!()
}
