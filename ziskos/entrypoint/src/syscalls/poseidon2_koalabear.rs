//! Poseidon2 KoalaBear system call interception

#[cfg(zisk_guest)]
use core::arch::asm;

#[cfg(zisk_guest)]
use crate::ziskos_syscall;

#[cfg(not(zisk_guest))]
use zisk_precomp_helpers::POSEIDON2_KOALABEAR;

/// Executes the width-16 Poseidon2 permutation over KoalaBear (`POSEIDON2_KOALABEAR` in
/// `zisk-precomp-helpers`) on the given state.
///
/// The state is sixteen KoalaBear elements in Montgomery form (`x * 2^32 mod P`, the
/// representation Plonky3 keeps them in), packed two per `u64` (little-endian), and is modified in
/// place. A lane that is not below P makes the execution fail.
///
/// ### Safety
///
/// The caller must ensure that the data is aligned to a 64-bit boundary.
#[allow(unused_variables)]
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_syscall_poseidon2_koalabear")]
pub unsafe extern "C" fn syscall_poseidon2_koalabear(
    state: *mut [u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    #[cfg(zisk_guest)]
    ziskos_syscall!(zisk_definitions::SYSCALL_POSEIDON2_KOALABEAR_ID, state);
    #[cfg(not(zisk_guest))]
    {
        let state: &mut [u64; 8] = unsafe { &mut *(state) };
        if let Err(lane) = POSEIDON2_KOALABEAR.permute_words(state) {
            panic!("syscall_poseidon2_koalabear: lane {lane} is not below the KoalaBear modulus");
        }

        #[cfg(feature = "hints")]
        {
            hints.extend_from_slice(state);
        }
    }
}
