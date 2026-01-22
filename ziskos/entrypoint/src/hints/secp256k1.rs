#[no_mangle]
pub unsafe extern "C" fn hint_secp256k1_ecrecover(sig: *const u8, recid: u8, msg: *const u8) {
    let sig_bytes: &[u8; 64] = &*(sig as *const [u8; 64]);
    let recid_bytes: [u8; 8] = (recid as u64).to_le_bytes();
    let msg_bytes: &[u8; 32] = &*(msg as *const [u8; 32]);

    let slice_bytes = {
        let mut buf = [0u8; (0 + 64 + 32)];
        let mut offset = 0;
        unsafe {
            core::ptr::copy_nonoverlapping(sig_bytes.as_ptr(), buf.as_mut_ptr().add(offset), sig_bytes.len());
        }
        offset += sig_bytes.len();
        unsafe {
            core::ptr::copy_nonoverlapping(recid_bytes.as_ptr(), buf.as_mut_ptr().add(offset), recid_bytes.len());
        }
        offset += recid_bytes.len();
        unsafe {
            core::ptr::copy_nonoverlapping(msg_bytes.as_ptr(), buf.as_mut_ptr().add(offset), msg_bytes.len());
        }
        buf
    };
    crate::hints::hint::hint_slice(0x0300, &slice_bytes, true);
}

#[cfg(zisk_hints_metrics)]
#[ctor::ctor]
fn secp256k1_ecrecover_register_meta() {
    crate::hints::register_hint(0x0300, stringify!(secp256k1_ecrecover).to_string());
}