use crate::hints::{HINT_QUEUE, check_main_thread, hint::{Hint, MAX_HINT_DATA_LEN}, macros::register_hint_meta};

const MODEXP_HINT_ID: u32 = 0x0500;

// Modular exponentiation hint
// Hint data layout: [base_len: 8 bytes][base_bytes: &[u8]][exp_len: 8 bytes][exp_bytes: &[u8]][modulus_len: 8 bytes][modulus_bytes: &[u8]]
#[no_mangle]
pub unsafe extern "C" fn hint_modexp_bytes_c(
    base_ptr: *const u8,
    base_len: usize,
    exp_ptr: *const u8,
    exp_len: usize,
    modulus_ptr: *const u8,
    modulus_len: usize,
) {
    if HINT_QUEUE.is_paused() {
        return;
    }

    check_main_thread();
    
    let base_bytes: &[u8] = unsafe { core::slice::from_raw_parts(base_ptr, base_len) };
    let exp_bytes: &[u8] = unsafe { core::slice::from_raw_parts(exp_ptr, exp_len) };
    let modulus_bytes: &[u8] = unsafe { core::slice::from_raw_parts(modulus_ptr, modulus_len) };

    assert!(
        base_len + exp_len + modulus_len + 24 <= MAX_HINT_DATA_LEN,
        "modexp hint data length exceeds MAX_HINT_DATA_LEN"
    );

    let mut hint = Hint::default();

    let mut offset = 0;
    unsafe {
        // Copy base length and base bytes
        let base_len_bytes: [u8; 8] = (base_len as u64).to_le_bytes();
        core::ptr::copy_nonoverlapping(base_len_bytes.as_ptr(), hint.data.as_mut_ptr(), 8);
        offset += 8;

        let len = base_bytes.len();
        core::ptr::copy_nonoverlapping(base_bytes.as_ptr(), hint.data.as_mut_ptr().add(offset), len);
        offset += len;

        // Copy exponent length and exponent bytes
        let exp_len_bytes: [u8; 8] = (exp_len as u64).to_le_bytes();
        core::ptr::copy_nonoverlapping(exp_len_bytes.as_ptr(), hint.data.as_mut_ptr().add(offset), 8);
        offset += 8;

        let len = exp_bytes.len();
        core::ptr::copy_nonoverlapping(exp_bytes.as_ptr(), hint.data.as_mut_ptr().add(offset), len);
        offset += len;

        // Copy modulus length and modulus bytes
        let modulus_len_bytes: [u8; 8] = (modulus_len as u64).to_le_bytes();
        core::ptr::copy_nonoverlapping(modulus_len_bytes.as_ptr(), hint.data.as_mut_ptr().add(offset), 8);
        offset += 8;

        let len = modulus_bytes.len();
        core::ptr::copy_nonoverlapping(modulus_bytes.as_ptr(), hint.data.as_mut_ptr().add(offset), len);
        offset += len;
    }

    hint.set_header(MODEXP_HINT_ID, offset, false);
    HINT_QUEUE.push(hint);
}

register_hint_meta!(modexp, MODEXP_HINT_ID);