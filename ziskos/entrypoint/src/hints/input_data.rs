use crate::hints::macros::define_hint_ptr;
use zisk_common::HINT_INPUT;

#[no_mangle]
pub unsafe extern "C" fn hint_input_data(input_data_ptr: *const u8, input_data_len: usize) {
    if !crate::hints::HINT_BUFFER.is_enabled() {
        return;
    }

    #[cfg(zisk_hints_single_thread)]
    if !crate::hints::check_main_thread() {
        return;
    }

    let pad = (8 - (input_data_len & 7)) & 7;
    let mut w = crate::hints::HINT_BUFFER.begin_input_data();

    // Write the length of the input data as the first 8 bytes of the hint data,
    // followed by the input data itself, and then pad with zeros if necessary
    let input_data_len_bytes: [u8; 8] = (input_data_len as u64).to_le_bytes();
    w.write_data_slice(&input_data_len_bytes);
    w.write_data_ptr(input_data_ptr, input_data_len);
    if pad > 0 {
        const ZERO_PAD: [u8; 8] = [0; 8];
        w.write_data_slice(&ZERO_PAD[..pad]);
    }
    w.commit();
}

#[cfg(zisk_hints_metrics)]
#[ctor::ctor]
fn input_data_register_meta() {
    crate::hints::metrics::register_hint(HINT_INPUT, stringify!(input_data).to_string());
}
