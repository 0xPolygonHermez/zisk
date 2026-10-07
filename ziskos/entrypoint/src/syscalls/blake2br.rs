//! Blake2br system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(not(zisk_guest))]
use zisk_precomp_helpers::blake2b_round;

/// Executes one `Blake2bRound` with a compile-time round `INDEX` (a number in [0,10)).
///
/// The precompiled instruction carries the round index as a static immediate, so the call site
/// has to name it at compile time. This is the dispatch-free entry point: `blake2b_compress`
/// unrolls its rounds over it, and [`syscall_blake2b_round`] keeps the runtime `index` API by
/// selecting the matching instantiation with a `match`.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[inline(always)]
pub fn syscall_blake2b_round_const<const INDEX: u64>(
    state: &mut [u64; 16],
    input: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID, a: state, b: input, imm: INDEX);

    #[cfg(not(zisk_guest))]
    {
        blake2b_round(state, input, INDEX as u32);

        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(state);
        }
    }
}

/// Executes the `Blake2bRound` operation, performing one round of the Blake2b compression function.
///
/// `Blake2bRound` operates on arrays of sixteen `u64` elements. It takes three parameters: `index`, `state`, and
/// `input`. The `index` parameter specifies which round to execute (a number in [0,10)); it is folded into the
/// precompiled instruction as a static argument, so this runtime-index entry point dispatches over the ten
/// instantiations of [`syscall_blake2b_round_const`]. Callers that know the index at compile time should use
/// that function directly and skip the dispatch.
/// The `state` parameter is a mutable reference to the current state of the Blake2b compression function, which will be updated in place.
/// The `input` parameter is a reference to the message block being processed.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_blake2b_round")]
pub extern "C" fn syscall_blake2b_round(
    index: u64,
    state: &mut [u64; 16],
    input: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    macro_rules! round {
        ($i:literal) => {
            syscall_blake2b_round_const::<$i>(
                state,
                input,
                #[cfg(feature = "hints")]
                hints,
            )
        };
    }
    match index {
        0 => round!(0),
        1 => round!(1),
        2 => round!(2),
        3 => round!(3),
        4 => round!(4),
        5 => round!(5),
        6 => round!(6),
        7 => round!(7),
        8 => round!(8),
        9 => round!(9),
        _ => panic!("syscall_blake2b_round: invalid round index {index}"),
    }
}
