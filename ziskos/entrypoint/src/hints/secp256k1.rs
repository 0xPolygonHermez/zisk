use crate::hints::{HINT_QUEUE, check_main_thread, hint::Hint, macros::{concat_hint_bytes, register_hint_meta}};

const SECP256K1_ECRECOVER_HINT_ID: u32 = 0x0300;

#[no_mangle]
pub unsafe extern "C" fn hint_secp256k1_ecrecover(sig: *const u8, recid: u8, msg: *const u8, require_low_s: bool) {
    if HINT_QUEUE.is_paused() {
        return;
    }

    check_main_thread();
    
    let sig_bytes: &[u8; 64] = &*(sig as *const [u8; 64]);
    let recid_bytes: &[u8; 8] = &((recid as u64).to_le_bytes());
    let msg_bytes: &[u8; 32] = &*(msg as *const [u8; 32]);
    let require_low_s_bytes: &[u8; 8] = &((require_low_s as u64).to_le_bytes());

    let slice_bytes = concat_hint_bytes!(64 + 8 + 32 + 8; sig_bytes, recid_bytes, msg_bytes, require_low_s_bytes);

    HINT_QUEUE.push(
        Hint::new(SECP256K1_ECRECOVER_HINT_ID, &slice_bytes, slice_bytes.len(), false)
    );
}

register_hint_meta!(secp256k1_ecrecover, SECP256K1_ECRECOVER_HINT_ID);
