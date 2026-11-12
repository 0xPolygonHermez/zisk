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
///
/// ### Panics
///
/// Instantiating this function with `INDEX >= 10` is a compile-time error: the BLAKE2b AIR only
/// accepts round indices in [0,10), so such a call could never be proved even though the host
/// implementation would silently reduce the index modulo 10.
#[allow(unused_variables)]
#[inline(always)]
pub fn syscall_blake2b_round_const<const INDEX: u64>(
    state: &mut [u64; 16],
    input: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    const {
        assert!(INDEX < 10, "syscall_blake2b_round_const: round index must be in [0,10)");
    }

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

#[cfg(zisk_guest)]
use zisk_definitions::{BLAKE2B_CHAIN_FIRST, BLAKE2B_CHAIN_LAST, BLAKE2B_CHAIN_MID};

/// Executes `rounds` rounds of the compression function. Runs of consecutive rounds are chained
/// (state and message pass to the next round without going through memory); each run is one asm
/// block, so the rounds are back to back, as the chain's step distance of 1 requires.
#[allow(unused_variables)]
pub fn syscall_blake2b_rounds_chain(
    rounds: u32,
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
    macro_rules! chain {
        (@pair $index:literal) => {
            "csrs {port}, {a}\naddi x0, {b}, {}"
        };
        ($first:literal $(, $mid:literal)*; $last:literal) => {{
            #[cfg(zisk_guest)]
            unsafe {
                core::arch::asm!(
                    chain!(@pair $first),
                    $( chain!(@pair $mid), )*
                    chain!(@pair $last),
                    const $first | BLAKE2B_CHAIN_FIRST,
                    $( const $mid | BLAKE2B_CHAIN_MID, )*
                    const $last | BLAKE2B_CHAIN_LAST,
                    port = const zisk_definitions::SYSCALL_BLAKE2B_ROUND_ID,
                    a = in(reg) state,
                    b = in(reg) input,
                    options(nostack)
                );
            }
            #[cfg(not(zisk_guest))]
            {
                round!($first);
                $( round!($mid); )*
                round!($last);
            }
        }};
    }

    let mut done = 0;
    while rounds - done >= 10 {
        chain!(0, 1, 2, 3, 4, 5, 6, 7, 8; 9);
        done += 10;
    }
    match rounds - done {
        0 => {}
        1 => round!(0),
        2 => chain!(0; 1),
        3 => chain!(0, 1; 2),
        4 => chain!(0, 1, 2; 3),
        5 => chain!(0, 1, 2, 3; 4),
        6 => chain!(0, 1, 2, 3, 4; 5),
        7 => chain!(0, 1, 2, 3, 4, 5; 6),
        8 => chain!(0, 1, 2, 3, 4, 5, 6; 7),
        _ => chain!(0, 1, 2, 3, 4, 5, 6, 7; 8),
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
