#![allow(unexpected_cfgs)]
#![allow(unused_imports)]

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod dma;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod fcall;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod alloc;

mod profile;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub use fcall::*;
pub mod io;
pub use profile::*;
pub mod syscalls;
pub mod zisklib;
pub mod ziskos_definitions;

#[cfg(all(
    not(all(target_os = "zkvm", target_vendor = "zisk")),
    any(zisk_hints, zisk_hints_debug),
    feature = "user-hints"
))]
pub mod hints;

#[cfg(all(not(all(target_os = "zkvm", target_vendor = "zisk")), zisk_hints))]
extern "C" {
    fn hint_input_data(input_data_ptr: *const u8, input_data_len: usize);
}

#[cfg(all(not(all(target_os = "zkvm", target_vendor = "zisk")), zisk_hints_debug))]
extern "C" {
    fn hint_log_c(msg: *const std::os::raw::c_char);
}

#[cfg(zisk_hints_debug)]
pub fn hint_log<S: AsRef<str>>(msg: S) {
    // On native we call external C function to log hints, since it controls if hints are paused or not
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    {
        use std::ffi::CString;

        if let Ok(c) = CString::new(msg.as_ref()) {
            unsafe { hint_log_c(c.as_ptr()) };
        }
    }
    // On zkvm/zisk, we can just print directly
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        println!("{}", msg.as_ref());
    }
}

#[macro_export]
macro_rules! entrypoint {
    ($path:path) => {
        const ZISK_ENTRY: fn() = $path;

        mod zkvm_generated_main {
            #[no_mangle]
            fn main() {
                super::ZISK_ENTRY()
            }
        }
    };
}

// #[macro_export]
// macro_rules! ziskos_fcall_get {
//     () => {{
//         read_csr_ffe()
//     }};
// }

#[allow(unused_imports)]
use crate::ziskos_definitions::ziskos_config::*;

/// Initial offset for input reading.
/// zkvm: 8 bytes offset due to INPUT_ADDR memory layout
/// native: 0 bytes offset (file starts at position 0)
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
const INPUT_INITIAL_OFFSET: usize = 8;
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
const INPUT_INITIAL_OFFSET: usize = 0;

/// Pointer to the current position in the input buffer/file.
static mut INPUT_POS: usize = INPUT_INITIAL_OFFSET;

/// Reset the input position to the beginning.
pub fn read_reset() {
    unsafe { INPUT_POS = INPUT_INITIAL_OFFSET };
}

/// Read a slice directly from INPUT_ADDR without copying (zero-copy).
///
/// This returns a slice pointing directly to the input memory region.
/// Use this when you want to deserialize directly without an intermediate copy.
/// The INPUT_POS is advanced after this call.
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub(crate) fn read_slice_zerocopy<'a>() -> &'a [u8] {
    // SAFETY: Single threaded, so nothing else can touch INPUT_POS while we're working.
    let input_pos = unsafe { INPUT_POS };
    let addr = (INPUT_ADDR as usize) + input_pos;

    // Ensure the 8-byte length prefix is ready and read it
    crate::zisklib::fcall_input_ready(&((addr + 7) as u64));
    let len = unsafe {
        let bytes = core::slice::from_raw_parts(addr as *const u8, 8);
        u64::from_le_bytes(bytes.try_into().unwrap()) as usize
    };

    // Ensure the data is ready (8-byte aligned)
    let data_addr = addr + 8;
    let aligned_len = (len + 7) & !0x7;
    crate::zisklib::fcall_input_ready(&((data_addr + aligned_len - 1) as u64));

    // Update input position: move past length (8 bytes) + data (8-byte aligned)
    unsafe { INPUT_POS = input_pos + 8 + aligned_len };

    let data_slice = unsafe { core::slice::from_raw_parts(data_addr as *const u8, len) };

    #[cfg(zisk_hints_debug)]
    {
        let start_bytes = &data_slice[..data_slice.len().min(64)];
        let ellipsis = if data_slice.len() > 64 { "..." } else { "" };
        hint_log(format!(
            "hint_input_data (input_data: {:x?}{} , input_data_len: {}",
            start_bytes,
            ellipsis,
            data_slice.len()
        ));
    }

    data_slice
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub(crate) fn read_input() -> Vec<u8> {
    read_slice_zerocopy().to_vec()
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
pub(crate) fn read_input() -> Vec<u8> {
    use std::{
        fs::File,
        io::{Read, Seek, SeekFrom},
    };

    let input_pos = unsafe { INPUT_POS };

    let mut file =
        File::open("build/input.bin").expect("Error opening input file at: build/input.bin");

    // Seek to the current position
    file.seek(SeekFrom::Start(input_pos as u64)).expect("Failed to seek in input file");

    // Read the 8-byte length prefix
    let mut len_bytes = [0u8; 8];
    file.read_exact(&mut len_bytes).expect("Failed to read length prefix from input file");
    let len = u64::from_le_bytes(len_bytes) as usize;

    // Read the actual data
    let mut data = vec![0u8; len];
    file.read_exact(&mut data).expect("Failed to read data from input file");

    // Advance INPUT_POS: 8 bytes for length + 8-byte aligned data
    let aligned_len = (len + 7) & !0x7;
    unsafe {
        INPUT_POS = input_pos + 8 + aligned_len;
    }

    #[cfg(zisk_hints)]
    unsafe {
        hint_input_data(data.as_ptr(), data.len());
    }

    #[cfg(zisk_hints_debug)]
    {
        let start_bytes = &data[..data.len().min(64)];
        let ellipsis = if data.len() > 64 { "..." } else { "" };
        hint_log(format!(
            "hint_input_data (input_data: {:x?}{} , input_data_len: {})",
            start_bytes,
            ellipsis,
            data.len()
        ));
    }

    data
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub(crate) fn set_output(id: usize, value: u32) {
    use std::arch::asm;
    let addr_v: *mut u32;
    let arch_id_zisk: usize;

    unsafe {
        asm!(
          "csrr {0}, marchid",
          out(reg) arch_id_zisk,
        )
    };

    assert!(id < 64, "Maximum number of public outputs: 64");

    if arch_id_zisk == ARCH_ID_ZISK as usize {
        addr_v = (OUTPUT_ADDR + 4 * (id as u64)) as *mut u32;
    } else {
        addr_v = (0x1000_0000 + 4 * (id as u64)) as *mut u32;
    }

    unsafe { core::ptr::write_volatile(addr_v, value) };
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
pub(crate) fn set_output(id: usize, value: u32) {
    println!("public {id}: {value:#010x}");
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub mod ziskos {
    use crate::ziskos_definitions::ziskos_config::*;
    use core::arch::asm;

    #[no_mangle]
    #[link_section = ".text.init"]
    unsafe extern "C" fn _start() -> ! {
        asm!(
          // before we use the `la` pseudo-instruction for the first time,
          //  we need to set `gp` (google linker relaxation)
          ".option push",
          ".option norelax",
          "la gp, _global_pointer",
          ".option pop",

          // set the stack pointer
          "la sp, _init_stack_top",

          // "tail-call" to {entry}
          "call {_zisk_main}",
          "csrr t0, marchid",
          //"li   t1, {_ARCH_ID_ZISK}",
          "li   t1, 0xFFFEEEE",
          "beq t0, t1, 1f",

          // QEmuu exit
          //"li t0, {_QEMU_EXIT_ADDR}",
          //"li t1, {_QEMU_EXIT_CODE}",
          "li t0, 0x100000",
          "li t1, 0x5555",
          "sw t1, 0(t0)",
          "j 2f",

          // Zisk exit
          "1: li   a7, 93",
          "ecall",

          "2: j 2b",

          _zisk_main = sym _zisk_main, // {entry} refers to the function [entry] below
          options(noreturn) // we must handle "returning" from assembly
        );

        pub fn zkvm_getrandom(s: &mut [u8]) -> Result<(), getrandom::Error> {
            unsafe {
                sys_rand(s.as_mut_ptr(), s.len());
            }

            Ok(())
        }

        getrandom::register_custom_getrandom!(zkvm_getrandom);
    }

    #[no_mangle]
    unsafe extern "C" fn _zisk_main() {
        {
            extern "C" {
                fn main();
            }
            #[cfg(any(
                feature = "zisk-embedded-alloc",
                feature = "zisk-embedded-dlmalloc-alloc",
                feature = "zisk-embedded-talc-alloc",
                feature = "zisk-embedded-tlfs-alloc"
            ))]
            crate::alloc::embedded::init();
            #[cfg(all(
                not(feature = "zisk-embedded-alloc"),
                not(feature = "zisk-embedded-dlmalloc-alloc"),
                not(feature = "zisk-embedded-talc-alloc"),
                not(feature = "zisk-embedded-tlfs-alloc")
            ))]
            crate::alloc::init_sys_alloc();

            main()
        }
    }

    #[no_mangle]
    pub extern "C" fn sys_write(_fd: u32, write_ptr: *const u8, nbytes: usize) {
        let arch_id_zisk: usize;
        let mut addr: *mut u8 = 0x1000_0000 as *mut u8;

        unsafe {
            asm!(
              "csrr {0}, marchid",
              out(reg) arch_id_zisk,
            )
        };
        if arch_id_zisk == ARCH_ID_ZISK as usize {
            addr = UART_ADDR as *mut u8;
        }

        for i in 0..nbytes {
            unsafe {
                core::ptr::write_volatile(addr, *write_ptr.add(i));
            }
        }
    }
    use lazy_static::lazy_static;
    use std::sync::Mutex;
    const PRNG_SEED: u64 = 0x123456789abcdef0;
    use rand::{rngs::StdRng, Rng, SeedableRng};

    lazy_static! {
        /// A lazy static to generate a global random number generator.
        static ref RNG: Mutex<StdRng> = Mutex::new(StdRng::seed_from_u64(PRNG_SEED));
    }

    /// A lazy static to print a warning once for using the `sys_rand` system call.
    static SYS_RAND_WARNING: std::sync::Once = std::sync::Once::new();

    #[no_mangle]
    unsafe extern "C" fn sys_rand(recv_buf: *mut u8, words: usize) {
        SYS_RAND_WARNING.call_once(|| {
            println!("WARNING: Using insecure random number generator.");
        });
        let mut rng = RNG.lock().unwrap();
        for i in 0..words {
            let element = recv_buf.add(i);
            *element = rng.gen();
        }
    }

    #[no_mangle]
    extern "C" fn sys_getenv() {
        //unimplemented!("sys_getenv")
    }

    #[no_mangle]
    extern "C" fn sys_alloc_words() {
        //unimplemented!("sys_alloc_words")
    }

    #[no_mangle]
    extern "C" fn sys_argc() {
        unimplemented!("sys_argc");
    }

    #[no_mangle]
    extern "C" fn sys_argv() {
        unimplemented!("sys_argv");
    }

    pub extern "C" fn sys_print_hex(val: usize, ln: bool) {
        let mut buf = [0u8; 19]; // "0x" + 16 hex + \n — stack, no heap
        buf[0] = b'0';
        buf[1] = b'x';
        let mut v = val;
        for i in (2..18).rev() {
            buf[i] = b"0123456789abcdef"[v & 0xF];
            v >>= 4;
        }
        if ln {
            buf[18] = b'\n';
            sys_write(1, buf.as_ptr(), buf.len());
        } else {
            sys_write(1, buf.as_ptr(), buf.len() - 1);
        }
    }

    core::arch::global_asm!(include_str!("dma/memcpy.s"));
    core::arch::global_asm!(include_str!("dma/memmove.s"));
    core::arch::global_asm!(include_str!("dma/memcmp.s"));
    //core::arch::global_asm!(include_str!("dma/inputcpy.s"));
    core::arch::global_asm!(include_str!("dma/memset.s"));
}
