#[inline]
pub fn concat_2_u64x4(a: &[u64; 4], b: &[u64; 4]) -> [u64; 8] {
    let mut buffer = [0u64; 8];
    unsafe {
        let ptr = buffer.as_mut_ptr();
        core::ptr::copy_nonoverlapping(a.as_ptr(), ptr, 4);
        core::ptr::copy_nonoverlapping(b.as_ptr(), ptr.add(4), 4);
    }
    buffer
}

#[inline]
pub fn concat_3_u64x4(a: &[u64; 4], b: &[u64; 4], c: &[u64; 4]) -> [u64; 12] {
    let mut buffer = [0u64; 12];
    unsafe {
        let ptr = buffer.as_mut_ptr();
        core::ptr::copy_nonoverlapping(a.as_ptr(), ptr, 4);
        core::ptr::copy_nonoverlapping(b.as_ptr(), ptr.add(4), 4);
        core::ptr::copy_nonoverlapping(c.as_ptr(), ptr.add(8), 4);
    }
    buffer
}
