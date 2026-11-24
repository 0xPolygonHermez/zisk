//! Memory lent to the allocator for a while.
//!
//! A component that owns a range of resident pages it does not need for now (the assembly
//! memory-operations output, once a run has been consumed) can [`lend`] it. Until it is
//! [taken back](take_back), [`lent_vec`] builds large buffers in it rather than in new memory:
//! the pages are already resident, so filling them costs no page faults and no memory the OS
//! still has to find. When the range is full, [`lent_vec`] falls back to the global allocator.
//!
//! The range is served by a jemalloc arena of its own, whose extent hooks hand out pieces of it
//! and never give them back to the OS. Only buffers built with [`lent_vec`] go there: the arena is
//! selected per allocation, never per thread, so no unrelated object lands in pages that will be
//! overwritten. Frees need nothing special: jemalloc returns a buffer larger than its thread-cache
//! limit straight to the arena it came from. A smaller one would stay in the freeing thread's
//! cache, which hands it to that thread's next allocation of its size, whatever it is, so those are
//! never lent.
//!
//! [`take_back`] gives the range back, destroying the arena, once no buffer lives in it. The owner
//! may write the range again only after that. A buffer still alive then is its owner's bug, as the
//! range is lent until the borrowers are done with it: [`take_back`] reports it and keeps the range
//! lent rather than wait for a free that may never come.

/// Why [`take_back`] could not give the range back. The range stays lent.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TakeBackError {
    /// Buffers built with [`lent_vec`] still live in the range.
    #[error("{bytes} bytes of buffers built in the lent memory are still alive")]
    BuffersAlive {
        /// Bytes they hold, in whole pages.
        bytes: u64,
    },
    /// jemalloc refused to destroy the arena serving the range.
    #[error("the arena serving the lent memory ({arena}) could not be destroyed")]
    ArenaNotDestroyed {
        /// The arena's index.
        arena: u32,
    },
    /// A panic while the lending state was held poisoned it.
    #[error("the lent memory's state is poisoned by a panic")]
    Poisoned,
}

#[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
mod imp {
    use std::os::raw::{c_char, c_uint, c_void};
    use std::ptr::{addr_of_mut, null_mut};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{OnceLock, RwLock};

    use tikv_jemalloc_sys as je;

    /// A range smaller than this is not lent: it would not hold the one extent the arena is primed
    /// with.
    const MIN_RANGE_BYTES: usize = 2 << 20;

    /// The lent range, `[BASE, END)`, handed out to the arena from `NEXT` upwards.
    static BASE: AtomicUsize = AtomicUsize::new(0);
    static END: AtomicUsize = AtomicUsize::new(0);
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    /// The arena serving the range while it is lent. An allocation holds the read side, so taking
    /// the range back (the write side) never races one into an arena being destroyed.
    static ARENA: RwLock<Option<c_uint>> = RwLock::new(None);

    /// The jemalloc releases the lending was written against. [`ArenaConfig`] mirrors by hand a
    /// struct of an experimental control, which another release is free to change: nothing is lent
    /// on one that has not been checked.
    const JEMALLOC_RELEASE: &str = "5.3.";

    /// Whether this is a jemalloc release the lending was written against ([`JEMALLOC_RELEASE`]).
    pub fn supported_release() -> bool {
        static SUPPORTED: OnceLock<bool> = OnceLock::new();
        *SUPPORTED.get_or_init(|| {
            let mut version: *const c_char = std::ptr::null();
            let mut len = std::mem::size_of::<*const c_char>();
            // SAFETY: `version` is a live output of the control's size, a pointer to a static string.
            let rc = unsafe {
                je::mallctl(
                    c"version".as_ptr(),
                    &mut version as *mut *const c_char as *mut c_void,
                    &mut len,
                    null_mut(),
                    0,
                )
            };
            if rc != 0 || version.is_null() {
                tracing::warn!("lent memory: jemalloc's version is unknown; not lending");
                return false;
            }
            // SAFETY: jemalloc's version string is static and NUL-terminated.
            let version = unsafe { std::ffi::CStr::from_ptr(version) }.to_string_lossy();
            let supported = version.starts_with(JEMALLOC_RELEASE);
            if !supported {
                tracing::warn!(
                    "lent memory: jemalloc {version} is not a {JEMALLOC_RELEASE}x release; not lending"
                );
            }
            supported
        })
    }

    /// jemalloc's `arena_config_t`, which the bindings do not export.
    #[repr(C)]
    struct ArenaConfig {
        extent_hooks: *mut je::extent_hooks_t,
        metadata_use_hooks: bool,
    }

    /// Only allocation hands out memory, from the range. Every way of giving memory back is opted
    /// out of (`None` hooks): freed pieces stay in the arena, committed, for its next buffers.
    /// Split and merge succeed, so pieces can be cut and joined again inside the range.
    static mut HOOKS: je::extent_hooks_t = je::extent_hooks_t {
        alloc: Some(range_alloc),
        dalloc: None,
        destroy: None,
        commit: Some(always_committed),
        decommit: None,
        purge_lazy: None,
        purge_forced: None,
        split: Some(allow_split),
        merge: Some(allow_merge),
    };

    /// Bump allocation over the range; `NULL` once it is exhausted, which fails the allocation in
    /// this arena (and only there: [`super::lent_vec`] then falls back to the global allocator).
    unsafe extern "C" fn range_alloc(
        _hooks: *mut je::extent_hooks_t,
        new_addr: *mut c_void,
        size: usize,
        alignment: usize,
        zero: *mut bool,
        commit: *mut bool,
        _arena: c_uint,
    ) -> *mut c_void {
        if !new_addr.is_null() {
            return null_mut();
        }
        let end = END.load(Ordering::Acquire);
        let mut next = NEXT.load(Ordering::Relaxed);
        loop {
            let start = (next + alignment - 1) & !(alignment - 1);
            let Some(stop) = start.checked_add(size) else {
                return null_mut();
            };
            if stop > end {
                return null_mut();
            }
            match NEXT.compare_exchange_weak(next, stop, Ordering::AcqRel, Ordering::Relaxed) {
                Ok(_) => {
                    // The pages hold whatever the owner wrote last: zero them only when asked.
                    if *zero {
                        std::ptr::write_bytes(start as *mut u8, 0, size);
                    }
                    *commit = true;
                    return start as *mut c_void;
                }
                Err(current) => next = current,
            }
        }
    }

    unsafe extern "C" fn always_committed(
        _hooks: *mut je::extent_hooks_t,
        _addr: *mut c_void,
        _size: usize,
        _offset: usize,
        _length: usize,
        _arena: c_uint,
    ) -> bool {
        false
    }

    unsafe extern "C" fn allow_split(
        _hooks: *mut je::extent_hooks_t,
        _addr: *mut c_void,
        _size: usize,
        _size_a: usize,
        _size_b: usize,
        _committed: bool,
        _arena: c_uint,
    ) -> bool {
        false
    }

    unsafe extern "C" fn allow_merge(
        _hooks: *mut je::extent_hooks_t,
        _addr_a: *mut c_void,
        _size_a: usize,
        _addr_b: *mut c_void,
        _size_b: usize,
        _committed: bool,
        _arena: c_uint,
    ) -> bool {
        false
    }

    fn mallctl_write<T>(name: &str, value: &mut T) -> bool {
        let Ok(name) = std::ffi::CString::new(name) else { return false };
        // SAFETY: `value` is a live, correctly sized input for the named control.
        unsafe {
            je::mallctl(
                name.as_ptr(),
                null_mut(),
                null_mut(),
                value as *mut T as *mut c_void,
                std::mem::size_of::<T>(),
            ) == 0
        }
    }

    /// A control that takes no input and gives no output (`arena.<i>.destroy`): any pointer
    /// passed, even to zero bytes, makes it fail.
    fn mallctl_command(name: &str) -> bool {
        let Ok(name) = std::ffi::CString::new(name) else { return false };
        // SAFETY: no input, no output.
        unsafe { je::mallctl(name.as_ptr(), null_mut(), null_mut(), null_mut(), 0) == 0 }
    }

    fn mallctl_read<T: Default>(name: &str) -> Option<T> {
        let name = std::ffi::CString::new(name).ok()?;
        let mut value = T::default();
        let mut len = std::mem::size_of::<T>();
        // SAFETY: `value` is a live output of the control's size.
        let rc = unsafe {
            je::mallctl(name.as_ptr(), &mut value as *mut T as *mut c_void, &mut len, null_mut(), 0)
        };
        (rc == 0).then_some(value)
    }

    /// Bytes of live allocations in `arena`, refreshed now.
    fn active_bytes(arena: c_uint) -> u64 {
        let mut epoch = 1u64;
        mallctl_write("epoch", &mut epoch);
        let page = mallctl_read::<usize>("arenas.page").unwrap_or(4096) as u64;
        let pages = mallctl_read::<usize>(&format!("stats.arenas.{arena}.pactive")).unwrap_or(0);
        pages as u64 * page
    }

    /// jemalloc's thread-cache limit (`arenas.tcache_max`, 32 KiB by default): only buffers larger
    /// than this are lent. A buffer up to it, freed through the global allocator, stays in the
    /// freeing thread's cache instead of going back to the lent arena, and the cache hands it to the
    /// thread's next allocation of that size: an unrelated object would live in the range, and
    /// taking it back would wait for that object. `None` when it cannot be read: nothing is lent.
    pub fn thread_cache_max() -> Option<usize> {
        static MAX: OnceLock<Option<usize>> = OnceLock::new();
        *MAX.get_or_init(|| mallctl_read::<usize>("arenas.tcache_max"))
    }

    /// What a lent buffer of `bytes` takes from the range: jemalloc's size class for it, plus the
    /// page a large allocation is padded with when `opt.cache_oblivious` is on (the default), as
    /// [`free_bytes`] counts it. `None` for a buffer [`alloc`] never lends.
    pub fn lent_size(bytes: usize) -> Option<usize> {
        if bytes <= thread_cache_max()? {
            return None;
        }
        // SAFETY: a pure size query.
        let size = unsafe { je::nallocx(bytes, 0) };
        (size != 0).then(|| size + large_pad())
    }

    fn large_pad() -> usize {
        static PAD: OnceLock<usize> = OnceLock::new();
        *PAD.get_or_init(|| {
            if mallctl_read::<bool>("opt.cache_oblivious").unwrap_or(true) {
                mallctl_read::<usize>("arenas.page").unwrap_or(4096)
            } else {
                0
            }
        })
    }

    pub fn lend(base: usize, len: usize) -> bool {
        let Ok(mut arena) = ARENA.write() else { return false };
        if arena.is_some() || len < MIN_RANGE_BYTES {
            return false;
        }
        if !supported_release() {
            return false;
        }
        if thread_cache_max().is_none() {
            tracing::warn!("lent memory: jemalloc's thread-cache limit is unknown; not lending");
            return false;
        }
        BASE.store(base, Ordering::Release);
        NEXT.store(base, Ordering::Release);
        END.store(base + len, Ordering::Release);

        // The arena's own metadata stays out of the range: it must outlive the range's reuse.
        let mut config =
            ArenaConfig { extent_hooks: addr_of_mut!(HOOKS), metadata_use_hooks: false };
        let mut index: c_uint = 0;
        let mut index_len = std::mem::size_of::<c_uint>();
        // SAFETY: `config` matches `arena_config_t`; `HOOKS` lives for the whole process.
        let rc = unsafe {
            je::mallctl(
                c"experimental.arenas_create_ext".as_ptr(),
                &mut index as *mut c_uint as *mut c_void,
                &mut index_len,
                &mut config as *mut ArenaConfig as *mut c_void,
                std::mem::size_of::<ArenaConfig>(),
            )
        };
        if rc != 0 {
            END.store(0, Ordering::Release);
            tracing::warn!("lent memory: could not create an arena (error {rc}); not lending");
            return false;
        }
        // Freed pieces go straight to the arena's retained extents. With no purge hooks nothing
        // reaches the OS, and a retained extent is cut for a request of any size and merged with
        // its free neighbours at once. Kept dirty instead, a free extent would only serve requests
        // at least 1/64 of its size (`opt.lg_extent_max_active_fit`), and the range would turn
        // away most buffers while nearly empty.
        let mut immediately: isize = 0;
        mallctl_write(&format!("arena.{index}.dirty_decay_ms"), &mut immediately);
        mallctl_write(&format!("arena.{index}.muzzy_decay_ms"), &mut immediately);

        // One allocation spanning most of the range, freed at once, leaves the arena the range as
        // one retained extent: later buffers are cut from it and merge back into it when freed.
        // Without it, every growth of the arena would ask the hook for a new extent, of sizes the
        // arena chooses, and the last ones would not fit what is left of the range.
        let flags = je::MALLOCX_ARENA(index as usize) | je::MALLOCX_TCACHE_NONE;
        let mut size = len & !((2 << 20) - 1);
        for _ in 0..32 {
            if size < MIN_RANGE_BYTES {
                break;
            }
            // SAFETY: plain allocation and free in the new arena.
            let ptr = unsafe { je::mallocx(size, flags) };
            if !ptr.is_null() {
                unsafe { je::dallocx(ptr, je::MALLOCX_TCACHE_NONE) };
                break;
            }
            size = size / 16 * 15;
        }
        *arena = Some(index);
        true
    }

    pub fn is_lent() -> bool {
        ARENA.read().map(|a| a.is_some()).unwrap_or(false)
    }

    pub fn free_bytes() -> u64 {
        let Ok(arena) = ARENA.read() else { return 0 };
        let Some(index) = *arena else { return 0 };
        let len = (END.load(Ordering::Acquire) - BASE.load(Ordering::Acquire)) as u64;
        len.saturating_sub(active_bytes(index))
    }

    pub fn alloc(bytes: usize, flags: i32) -> Option<*mut u8> {
        if bytes <= thread_cache_max()? {
            return None;
        }
        // Taking the range back holds the write side: allocate elsewhere meanwhile.
        let arena = ARENA.try_read().ok()?;
        let index = (*arena)?;
        let flags = flags | je::MALLOCX_ARENA(index as usize) | je::MALLOCX_TCACHE_NONE;
        // SAFETY: plain allocation in the lent arena.
        let ptr = unsafe { je::mallocx(bytes, flags) };
        (!ptr.is_null()).then_some(ptr as *mut u8)
    }

    pub fn take_back() -> Result<(), super::TakeBackError> {
        let Ok(mut arena) = ARENA.write() else { return Err(super::TakeBackError::Poisoned) };
        let Some(index) = *arena else { return Ok(()) };
        let bytes = active_bytes(index);
        if bytes > 0 {
            return Err(super::TakeBackError::BuffersAlive { bytes });
        }
        if !mallctl_command(&format!("arena.{index}.destroy")) {
            return Err(super::TakeBackError::ArenaNotDestroyed { arena: index });
        }
        *arena = None;
        END.store(0, Ordering::Release);
        NEXT.store(0, Ordering::Release);
        BASE.store(0, Ordering::Release);
        Ok(())
    }
}

/// Lends `[base, base + len)` to the allocator for [`lent_vec`] buffers. The pages must stay mapped,
/// writable and untouched by their owner until [`take_back`] returns. Returns whether it is lent;
/// `false` when something already is, or the allocator cannot.
pub fn lend(base: usize, len: usize) -> bool {
    #[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
    {
        imp::lend(base, len)
    }
    #[cfg(not(all(feature = "jemalloc", not(target_env = "msvc"))))]
    {
        let _ = (base, len);
        false
    }
}

/// Whether a range is lent now.
pub fn is_lent() -> bool {
    #[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
    {
        imp::is_lent()
    }
    #[cfg(not(all(feature = "jemalloc", not(target_env = "msvc"))))]
    {
        false
    }
}

/// Bytes of the lent range not held by live buffers: what [`lent_vec`] can still place there.
pub fn free_bytes() -> u64 {
    #[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
    {
        imp::free_bytes()
    }
    #[cfg(not(all(feature = "jemalloc", not(target_env = "msvc"))))]
    {
        0
    }
}

/// What a buffer of `bytes` built with [`lent_vec`] takes from a lent range, or `None` when it never
/// goes there (it is not larger than jemalloc's thread-cache limit, or nothing can be lent). The
/// figure [`free_bytes`] is measured in, so a budget can compare the two.
pub fn lent_size(bytes: usize) -> Option<usize> {
    #[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
    {
        imp::lent_size(bytes)
    }
    #[cfg(not(all(feature = "jemalloc", not(target_env = "msvc"))))]
    {
        let _ = bytes;
        None
    }
}

/// An empty `Vec` with room for `capacity` elements, in the lent range when one is lent, the buffer
/// is larger than jemalloc's thread-cache limit (32 KiB by default) and it fits; from the global
/// allocator otherwise. It behaves as any `Vec`: growing it past `capacity` moves it to the global
/// allocator.
pub fn lent_vec<T>(capacity: usize) -> Vec<T> {
    #[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
    {
        let bytes = capacity.saturating_mul(std::mem::size_of::<T>());
        // The same alignment flag the global allocator passes when it frees this buffer, so
        // jemalloc sizes the free as it sized the allocation.
        let align = std::mem::align_of::<T>();
        let flags =
            if align <= 16 && align <= bytes { 0 } else { tikv_jemalloc_sys::MALLOCX_ALIGN(align) };
        if let Some(ptr) = imp::alloc(bytes, flags) {
            // SAFETY: `ptr` is a jemalloc allocation of `capacity` elements of `T`, aligned for
            // `T`, above the thread-cache limit; the global allocator is jemalloc, which frees it
            // straight into its own arena.
            return unsafe { Vec::from_raw_parts(ptr as *mut T, 0, capacity) };
        }
    }
    Vec::with_capacity(capacity)
}

/// Gives the lent range back when no [`lent_vec`] buffer lives in it: once it returns `Ok`, the
/// owner may write the range again. `Ok` when nothing was lent. It does not wait: a buffer still
/// alive is an error, and the range stays lent.
pub fn take_back() -> Result<(), TakeBackError> {
    #[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
    {
        imp::take_back()
    }
    #[cfg(not(all(feature = "jemalloc", not(target_env = "msvc"))))]
    {
        Ok(())
    }
}

#[cfg(all(test, feature = "jemalloc", not(target_env = "msvc")))]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};

    /// The lending is process-wide, so one test walks the whole cycle.
    #[test]
    fn large_buffers_land_in_the_lent_range_until_it_is_taken_back() {
        let len = 64 << 20;
        let layout = Layout::from_size_align(len, 2 << 20).unwrap();
        // The range must not be jemalloc's own memory, whose extents it already maps by address,
        // so it comes from the system allocator, as the real one comes from a shared mapping.
        // SAFETY: a fresh allocation, lent and given back before it is freed.
        let base = unsafe { System.alloc(layout) } as usize;
        assert_ne!(base, 0);
        let inside = |p: usize, bytes: usize| p >= base && p + bytes <= base + len;

        // A new jemalloc release fails here: check `ArenaConfig` against it before lending on it.
        assert!(imp::supported_release(), "a jemalloc release the lending was written against");
        assert!(lend(base, len));
        assert!(is_lent());
        assert!(!lend(base, len), "one range at a time");

        let big: Vec<u64> = lent_vec((4 << 20) / 8);
        assert!(inside(big.as_ptr() as usize, 4 << 20), "a large buffer goes to the range");
        // Per-chunk sized buffers, far below 1/64 of the free range (a dirty extent would turn them
        // away), and more of them than any tail of the range the priming left to the hook.
        let medium: Vec<Vec<u8>> = (0..256).map(|_| lent_vec(64 << 10)).collect();
        for buffer in &medium {
            assert!(inside(buffer.as_ptr() as usize, 64 << 10), "so do per-chunk sized ones");
        }
        // One the thread cache would keep when freed stays with the global allocator.
        let cache_max = imp::thread_cache_max().unwrap();
        let small: Vec<u8> = lent_vec(cache_max);
        assert!(!inside(small.as_ptr() as usize, cache_max), "a cacheable one stays out");
        assert!(free_bytes() <= (len - (4 << 20)) as u64);

        // A lent buffer takes from the range exactly what `lent_size` says, size class and padding
        // included, so a budget can weigh one against the other.
        assert_eq!(lent_size(cache_max), None, "a cacheable one is never lent");
        let odd = (1 << 20) + 1;
        let free = free_bytes();
        let measured: Vec<u8> = lent_vec(odd);
        assert!(inside(measured.as_ptr() as usize, odd));
        assert_eq!(free - free_bytes(), lent_size(odd).unwrap() as u64);
        assert!(lent_size(odd).unwrap() > odd, "a size class above the request");
        drop(measured);

        // Every freed buffer is back in the range at once: none lingers in a thread cache.
        drop(big);
        drop(medium);
        assert_eq!(free_bytes(), len as u64, "freed buffers return to the range");

        // Freed space is cut again: two buffers of most of the range, one after the other.
        let most = len / 4 * 3;
        let first: Vec<u8> = lent_vec(most);
        assert!(inside(first.as_ptr() as usize, most));
        drop(first);
        let second: Vec<u8> = lent_vec(most);
        assert!(inside(second.as_ptr() as usize, most), "the freed space is used again");
        drop(second);
        assert_eq!(take_back(), Ok(()));
        assert!(!is_lent());
        let after: Vec<u64> = lent_vec((4 << 20) / 8);
        assert!(!inside(after.as_ptr() as usize, 4 << 20), "nothing goes to a range given back");

        drop(after);
        drop(small);

        // A buffer still alive keeps the range lent: taking it back fails at once, and succeeds
        // once the buffer is gone.
        assert!(lend(base, len));
        let held: Vec<u64> = lent_vec((4 << 20) / 8);
        assert!(inside(held.as_ptr() as usize, 4 << 20));
        assert!(
            matches!(take_back(), Err(TakeBackError::BuffersAlive { bytes }) if bytes >= 4 << 20),
            "a live buffer is reported"
        );
        assert!(is_lent(), "the range stays lent");
        drop(held);
        assert_eq!(take_back(), Ok(()));
        assert!(!is_lent());
        // SAFETY: allocated above with `layout`; nothing lives in it any more.
        unsafe { System.dealloc(base as *mut u8, layout) };
    }
}
