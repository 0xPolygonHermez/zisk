#[cfg(zisk_guest)]
use crate::ziskos_syscall;
#[cfg(zisk_guest)]
use core::arch::asm;

/// Permutes sixteen canonical KoalaBear lanes packed pairwise into eight u64 words.
///
/// # Safety
/// `state` must point to an initialized, writable, 8-byte-aligned 64-byte region.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_koala_poseidon2")]
pub unsafe extern "C" fn syscall_koala_poseidon2(
    state: *mut [u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    assert!(!state.is_null() && (state as usize) & 7 == 0, "invalid KoalaBear state pointer");
    assert!((state as usize).checked_add(63).is_some(), "KoalaBear state address overflow");
    #[cfg(all(zisk_hints, not(feature = "hints")))]
    unsafe {
        crate::hints::hint_koala_poseidon2(state.cast());
    }
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_KOALA_POSEIDON2_ID, state);
    #[cfg(not(zisk_guest))]
    {
        let state = unsafe { &mut *state };
        zisk_definitions::koala_poseidon2::permute_packed(state)
            .expect("noncanonical KoalaBear precompile input");
        #[cfg(feature = "hints")]
        if zisk_definitions::KOALA_POSEIDON2_RESULTS {
            hints.extend_from_slice(state);
        }
    }
}
