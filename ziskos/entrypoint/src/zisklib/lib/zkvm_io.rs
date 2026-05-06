//! Standard zkVM IO functions implementing the C interface from zkvm_io.h.
//!
//! ZisK stdin is stored as length-prefixed records. The standard IO interface is
//! exposed as the first logical input record and is idempotent. Guests should use
//! either this standard IO interface or ZisK's streaming input APIs for a given
//! input, not both: standard reads do not advance ZisK's streaming input cursor.

use core::ptr::{self, addr_of, addr_of_mut};

// Public outputs are written as u32 slots via set_output.
const OUTPUT_WORD_SIZE: usize = core::mem::size_of::<u32>();

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
static STANDARD_INPUT: std::sync::Mutex<Option<&'static [u8]>> = std::sync::Mutex::new(None);

static mut OUTPUT_WORD_SLOT: usize = 0;
static mut OUTPUT_PENDING: [u8; OUTPUT_WORD_SIZE] = [0; OUTPUT_WORD_SIZE];
static mut OUTPUT_PENDING_LEN: usize = 0;

/// # Safety
///
/// `buf_ptr` and `buf_size` must be valid writable pointers.
///
/// This function is idempotent and does not advance ZisK's streaming input
/// cursor. Mixing it with streaming reads may expose the first input record more
/// than once.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_read_input")]
pub unsafe extern "C" fn read_input(buf_ptr: *mut *const u8, buf_size: *mut usize) {
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        let (data_ptr, len) = zkvm_standard_input();
        ptr::write(buf_ptr, data_ptr);
        ptr::write(buf_size, len);
    }
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        let mut input = STANDARD_INPUT.lock().unwrap();
        if input.is_none() {
            let saved_pos = unsafe { crate::INPUT_POS };
            unsafe { crate::INPUT_POS = crate::INPUT_INITIAL_OFFSET };
            let data: &'static [u8] = Box::leak(crate::read_input().into_boxed_slice());
            unsafe { crate::INPUT_POS = saved_pos };
            *input = Some(data);
        }
        let data = input.expect("standard input initialized");
        ptr::write(buf_ptr, if data.is_empty() { ptr::null() } else { data.as_ptr() });
        ptr::write(buf_size, data.len());
    }
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
fn zkvm_standard_input() -> (*const u8, usize) {
    static mut INPUT_PTR: *const u8 = ptr::null();
    static mut INPUT_LEN: usize = 0;
    static mut INPUT_READY: bool = false;

    unsafe {
        if !INPUT_READY {
            let addr = (crate::ziskos_definitions::ziskos_config::INPUT_ADDR as usize)
                + crate::INPUT_INITIAL_OFFSET;

            crate::zisklib::fcall_input_ready(&((addr + 7) as u64));
            let len = {
                let bytes = core::slice::from_raw_parts(addr as *const u8, 8);
                u64::from_le_bytes(bytes.try_into().unwrap()) as usize
            };

            let data_addr = addr + 8;
            if len > 0 {
                let last_byte_addr = data_addr + len - 1;
                crate::zisklib::fcall_input_ready(&(last_byte_addr as u64));
                INPUT_PTR = data_addr as *const u8;
            } else {
                INPUT_PTR = ptr::null();
            }
            INPUT_LEN = len;
            INPUT_READY = true;
        }

        (INPUT_PTR, INPUT_LEN)
    }
}

/// # Safety
///
/// If `size > 0`, `output` must point to at least `size` readable bytes.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_write_output")]
pub unsafe extern "C" fn write_output(output: *const u8, size: usize) {
    if size == 0 {
        return;
    }

    let mut ptr = output;
    let mut remaining = size;

    while OUTPUT_PENDING_LEN != 0 && remaining != 0 {
        OUTPUT_PENDING[OUTPUT_PENDING_LEN] = ptr::read(ptr);
        OUTPUT_PENDING_LEN += 1;
        ptr = ptr.add(1);
        remaining -= 1;

        write_pending_word();
    }

    while remaining >= OUTPUT_WORD_SIZE {
        let value = u32::from_le_bytes(ptr::read_unaligned(ptr as *const [u8; OUTPUT_WORD_SIZE]));
        crate::set_output(OUTPUT_WORD_SLOT, value);
        OUTPUT_WORD_SLOT += 1;
        ptr = ptr.add(OUTPUT_WORD_SIZE);
        remaining -= OUTPUT_WORD_SIZE;
    }

    if remaining != 0 {
        ptr::copy_nonoverlapping(ptr, addr_of_mut!(OUTPUT_PENDING) as *mut u8, remaining);
        OUTPUT_PENDING_LEN = remaining;
    }

    if OUTPUT_PENDING_LEN != 0 {
        write_padded_pending_word();
    }
}

unsafe fn write_pending_word() {
    if OUTPUT_PENDING_LEN == OUTPUT_WORD_SIZE {
        crate::set_output(OUTPUT_WORD_SLOT, u32::from_le_bytes(OUTPUT_PENDING));
        OUTPUT_WORD_SLOT += 1;
        OUTPUT_PENDING_LEN = 0;
    }
}

unsafe fn write_padded_pending_word() {
    let mut bytes = [0u8; OUTPUT_WORD_SIZE];
    ptr::copy_nonoverlapping(
        addr_of!(OUTPUT_PENDING) as *const u8,
        bytes.as_mut_ptr(),
        OUTPUT_PENDING_LEN,
    );
    crate::set_output(OUTPUT_WORD_SLOT, u32::from_le_bytes(bytes));
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
pub(crate) fn reset() {
    // Leaked slices are intentionally not freed; each reset() will re-read
    // fresh input on the next call, leaking one allocation per test run.
    *STANDARD_INPUT.lock().unwrap() = None;

    reset_output();
}

pub(crate) fn reset_output() {
    unsafe {
        OUTPUT_WORD_SLOT = 0;
        OUTPUT_PENDING = [0; OUTPUT_WORD_SIZE];
        OUTPUT_PENDING_LEN = 0;
    }
}

#[cfg(not(feature = "hints"))]
#[allow(dead_code)]
mod _interface_type_checks {
    use super::*;
    use zkvm_interface as bindings;

    fn _check() {
        let _ = [bindings::read_input, super::read_input];
        let _ = [bindings::write_output, super::write_output];
    }
}
