use crate::hints::{HINT_QUEUE, hint::{Hint, MAX_HINT_DATA_LEN}, macros::{concat_hint_bytes, register_hint_meta}};

const BLS12_381_G1_ADD_HINT_ID: u32 = 0x0400;
const BLS12_381_G1_MSM_HINT_ID: u32 = 0x0401;
const BLS12_381_G2_ADD_HINT_ID: u32 = 0x0405;

crate::hints::macros::define_hint! {
    bls12_381_g1_add => {
        hint_id: BLS12_381_G1_ADD_HINT_ID,
        params: (a: 96, b: 96),
        is_result: false,
    }
}

// BLS12-381 G1 MSM hint
// Hint data layout: [num_pairs: 8 bytes][point_1: 96 bytes][scalar_1: 32 bytes]...[point_n: 96 bytes][scalar_n: 32 bytes]
#[no_mangle]
pub unsafe extern "C" fn hint_bls12_381_g1_msm(pairs: *const u8, num_pairs: usize) {
    let mut hint = Hint::default();

    let total_len: u64 = num_pairs as u64 * (96 + 32);
    assert!(
        total_len as usize <= MAX_HINT_DATA_LEN,
        "bls12_381_g1_msm hint data length exceeds MAX_HINT_DATA_LEN"
    );

    let mut offset = 0;

    unsafe {
        let num_pairs_bytes: [u8; 8] = (num_pairs as u64).to_le_bytes();
        core::ptr::copy_nonoverlapping(num_pairs_bytes.as_ptr(), hint.data.as_mut_ptr(), 8);
        offset += 8;
    }

    for i in 0..num_pairs {
        let pair_ptr = pairs.add(i * 128);

        // Extract point (96 bytes) and scalar (32 bytes)
        let point_bytes: &[u8; 96] = &*(pair_ptr as *const [u8; 96]);
        let scalar_bytes: &[u8; 32] = &*(pair_ptr.add(96) as *const [u8; 32]);

        concat_hint_bytes!(offset; 96 + 32; point_bytes, scalar_bytes);
    }

    hint.set_header(BLS12_381_G1_MSM_HINT_ID, offset, false);
    HINT_QUEUE.push(hint);
}

register_hint_meta!(bls12_381_g1_msm, BLS12_381_G1_MSM_HINT_ID);

crate::hints::macros::define_hint! {
    bls12_381_g2_add => {
        hint_id: BLS12_381_G2_ADD_HINT_ID,
        params: (a: 192, b: 192),
        is_result: false,
    }
}
