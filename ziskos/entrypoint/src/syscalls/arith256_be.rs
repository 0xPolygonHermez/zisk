//! Arith256 (big-endian) system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

use super::arith256::SyscallArith256Params;

/// Big-endian variant of [`syscall_arith256`](super::syscall_arith256).
///
/// Identical to `syscall_arith256`, except that all 256-bit operands and results
/// (`a`, `b`, `c`, `dl`, `dh`) are expected/produced with their limb order reversed
/// and each limb stored in big-endian byte order.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_arith256_be")]
pub extern "C" fn syscall_arith256_be(
    params: &mut SyscallArith256Params,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_ARITH256_BE_ID, params);
    #[cfg(not(zisk_guest))]
    {
        use super::be_utils::be_swap_4;
        let a = be_swap_4(params.a);
        let b = be_swap_4(params.b);
        let c = be_swap_4(params.c);
        let mut dl = [0u64; 4];
        let mut dh = [0u64; 4];
        precompiles_helpers::arith256(&a, &b, &c, &mut dl, &mut dh);
        let dl_be = be_swap_4(&dl);
        let dh_be = be_swap_4(&dh);
        params.dl.copy_from_slice(&dl_be);
        params.dh.copy_from_slice(&dh_be);
        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(params.dl);
            hints.extend_from_slice(params.dh);
        }
    }
}
