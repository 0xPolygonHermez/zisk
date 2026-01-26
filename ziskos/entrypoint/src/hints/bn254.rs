use crate::hints::{HINT_QUEUE, check_main_thread, hint::{Hint, MAX_HINT_DATA_LEN}, macros::{concat_hint_bytes, register_hint_meta}};

const BN254_G1_ADD_HINT_ID: u32 = 0x0200;
const BN254_G1_MUL_HINT_ID: u32 = 0x0201;
const BN254_PAIRING_CHECK_HINT_ID: u32 = 0x0205;

crate::hints::macros::define_hint! {
    bn254_g1_add => {
        hint_id: BN254_G1_ADD_HINT_ID,
        params: (p1: 64, p2: 64),
        is_result: false,
    }
}


crate::hints::macros::define_hint! {
    bn254_g1_mul => {
        hint_id: BN254_G1_MUL_HINT_ID,
        params: (point: 64, scalar: 32),
        is_result: false,
    }
}

// BN254 pairing check hint
// Hint data layout: [num_pairs: 8 bytes][g1_point_1: 64 bytes][g2_point_1: 128 bytes]...[g1_point_n: 64 bytes][g2_point_n: 128 bytes]
#[no_mangle]
pub unsafe extern "C" fn hint_bn254_pairing_check(pairs: *const u8, num_pairs: usize) {
    if HINT_QUEUE.is_paused() {
        return;
    }

    check_main_thread();

    let mut hint = Hint::default();

    let total_len: u64 = 8 + (num_pairs as u64 * (64 + 128));
    assert!(
        total_len as usize <= MAX_HINT_DATA_LEN,
        "bn254_pairing_check hint data length exceeds MAX_HINT_DATA_LEN"
    );

    let mut offset = 0;

    unsafe {
        let num_pairs_bytes: [u8; 8] = (num_pairs as u64).to_le_bytes();
        core::ptr::copy_nonoverlapping(num_pairs_bytes.as_ptr(), hint.data.as_mut_ptr(), 8);
        offset += 8;
    }

    for i in 0..num_pairs {
        let pair_ptr = pairs.add(i * 64 + 128);

        let g1_bytes: &[u8; 64] = &*(pair_ptr as *const [u8; 64]);
        let g2_bytes: &[u8; 128] = &*(pair_ptr.add(64) as *const [u8; 128]);

        let pair = concat_hint_bytes!(64 + 128; g1_bytes, g2_bytes);

        unsafe {
            core::ptr::copy_nonoverlapping(pair.as_ptr(), hint.data.as_mut_ptr().add(offset), 64 + 128);
        }

        offset += 64 + 128;
    }

    hint.set_header(BN254_PAIRING_CHECK_HINT_ID, offset, false);
    HINT_QUEUE.push(hint);
}

register_hint_meta!(bn254_pairing_check, BN254_PAIRING_CHECK_HINT_ID);