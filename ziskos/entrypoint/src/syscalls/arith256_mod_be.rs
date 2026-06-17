//! Arith256Mod (big-endian) system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

use super::arith256_mod::SyscallArith256ModParams;

/// Big-endian variant of [`syscall_arith256_mod`](super::syscall_arith256_mod).
///
/// Identical to `syscall_arith256_mod`, except that all 256-bit operands and the
/// result (`a`, `b`, `c`, `module`, `d`) are expected/produced with their limb
/// order reversed and each limb stored in big-endian byte order.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
///
/// The caller must ensure that `module` is not zero.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_arith256_mod_be")]
pub extern "C" fn syscall_arith256_mod_be(
    params: &mut SyscallArith256ModParams,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_ARITH256_MOD_BE_ID, params);
    #[cfg(not(zisk_guest))]
    {
        use super::be_utils::be_swap_4;
        let a = be_swap_4(params.a);
        let b = be_swap_4(params.b);
        let c = be_swap_4(params.c);
        let module = be_swap_4(params.module);
        let mut d = [0u64; 4];
        precompiles_helpers::arith256_mod(&a, &b, &c, &module, &mut d);
        let d_be = be_swap_4(&d);
        params.d.copy_from_slice(&d_be);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.d);
        }
    }
}
