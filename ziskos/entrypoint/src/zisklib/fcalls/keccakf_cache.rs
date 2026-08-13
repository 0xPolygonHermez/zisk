use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(zisk_guest)] {
        use core::arch::asm;
        use crate::{ziskos_fcall, ziskos_fcall_get, ziskos_fcall_param};
        use super::{FCALL_GET_KECCAKF_CACHE_INDEX_ID, FCALL_SET_KECCAKF_CACHE_INDEX_ID};
    } else {
        use crate::zisklib::fcalls_impl::keccakf_cache::{
            keccakf_cache_get_index, keccakf_cache_set_index,
        };
    }
}

/// Asks the executor to cache the input state of the *next* Keccak-f it runs under `index`.
///
/// The 25 words the permutation reads are added to the executor's cache, so a later
/// [`fcall_get_keccakf_cache_index`] with the same state returns `index`. Call it right
/// before [`syscall_keccak_f`](crate::syscalls::syscall_keccak_f); any Keccak-f consumes the
/// request, and a request that is never followed by one is simply dropped when the execution
/// ends.
///
/// `index` must not be [`KECCAKF_CACHE_INDEX_NOT_FOUND`](super::KECCAKF_CACHE_INDEX_NOT_FOUND),
/// which is reserved to report a miss.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the
/// correctness of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_set_keccakf_cache_index(index: u64) {
    #[cfg(not(zisk_guest))]
    {
        keccakf_cache_set_index(index);
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(index, 1);
        ziskos_fcall!(FCALL_SET_KECCAKF_CACHE_INDEX_ID);
    }
}

/// Returns the index `state` was cached under by a previous
/// [`fcall_set_keccakf_cache_index`], or
/// [`KECCAKF_CACHE_INDEX_NOT_FOUND`](super::KECCAKF_CACHE_INDEX_NOT_FOUND) if the executor has
/// not seen this Keccak-f input state.
///
/// Note that this is a *free-input call*, meaning the ZisK VM does not automatically verify the
/// correctness of the result. It is the caller's responsibility to ensure it.
#[allow(unused_variables)]
pub fn fcall_get_keccakf_cache_index(state: &[u64; 25]) -> u64 {
    #[cfg(not(zisk_guest))]
    {
        keccakf_cache_get_index(state)
    }
    #[cfg(zisk_guest)]
    {
        ziskos_fcall_param!(state, 25);
        ziskos_fcall!(FCALL_GET_KECCAKF_CACHE_INDEX_ID);
        ziskos_fcall_get()
    }
}
