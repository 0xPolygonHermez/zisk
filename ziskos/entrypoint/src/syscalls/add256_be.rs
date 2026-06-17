//! Add256 (big-endian) system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall_ret_u64;

use super::add256::SyscallAdd256Params;

/// Big-endian variant of [`syscall_add256`](super::syscall_add256).
///
/// Identical to `syscall_add256`, except that the 256-bit operands `a`, `b` and the
/// result `c` are expected/produced with their limb order reversed and each limb
/// stored in big-endian byte order. The carry input `cin` and the carry output
/// `cout` remain in native little-endian representation.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_add256_be")]
pub extern "C" fn syscall_add256_be(
    params: &mut SyscallAdd256Params,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u64 {
    #[cfg(not(zisk_guest))]
    {
        use super::be_utils::be_swap_4;
        let a = be_swap_4(params.a);
        let b = be_swap_4(params.b);
        let mut c = [0u64; 4];
        let cout = precompiles_helpers::add256(&a, &b, params.cin, &mut c);
        let c_be = be_swap_4(&c);
        params.c.copy_from_slice(&c_be);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.c);
            hints.push(cout);
        }
        cout
    }
    #[cfg(zisk_guest)]
    ziskos_syscall_ret_u64!(zisk_definitions::SYSCALL_ADD256_BE_ID, params)
}
