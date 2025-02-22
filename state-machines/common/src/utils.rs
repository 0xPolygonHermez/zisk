use std::{mem::MaybeUninit, sync::atomic::AtomicU64};

pub fn create_atomic_vec(size: usize) -> Vec<AtomicU64> {
    let mut vec: Vec<MaybeUninit<AtomicU64>> = Vec::with_capacity(size);

    unsafe {
        let ptr = vec.as_mut_ptr() as *mut u8;
        std::ptr::write_bytes(ptr, 0, size * std::mem::size_of::<AtomicU64>()); // Fast zeroing

        vec.set_len(size);
        std::mem::transmute(vec) // Convert MaybeUninit<Vec> -> Vec<AtomicU64>
    }
}
