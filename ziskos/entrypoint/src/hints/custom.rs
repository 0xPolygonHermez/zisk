use crate::hints::HINT_BUFFER;

#[no_mangle]
pub unsafe extern "C" fn hint_custom(
    hint_id: u32,
    data_ptr: *const u8,
    data_len: usize,
    is_result: u8,
) {
    if !HINT_BUFFER.is_enabled() {
        return;
    }

    #[cfg(zisk_hints_single_thread)]
    if !crate::hints::check_main_thread() {
        return;
    }

    let mut w = HINT_BUFFER.begin_hint(hint_id, data_len, is_result != 0);

    w.write_data_ptr(data_ptr, data_len);

    let pad = (8 - (data_len & 7)) & 7;
    if pad > 0 {
        const ZERO_PAD: [u8; 8] = [0; 8];
        w.write_data_slice(&ZERO_PAD[..pad]);
    }

    w.commit();
}
