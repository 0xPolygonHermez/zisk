#![cfg_attr(zisk_guest, no_std)]
// `linkage` is needed for the weak C++ runtime symbols (`__cxa_atexit`,
// `__cxa_finalize`, `__dso_handle`) declared in `_zisk_main`. The linker-provided
// init/fini array bounds are declared as *strong* externs (a weak static would be
// reached through a hidden pointer slot, so `addr_of!` would not yield the section
// address); standard linkers (GNU ld, LLD) always define those symbols. Guest-only,
// so native/stable builds of this crate are unaffected.
#![cfg_attr(zisk_guest, feature(linkage))]
#![allow(unexpected_cfgs)]
#![allow(unused_imports)]

#[cfg(zisk_guest)]
use core::arch::asm;
#[cfg(zisk_guest)]
mod dma;
#[cfg(zisk_guest)]
mod fcall;

#[cfg(zisk_guest)]
mod alloc;

// Link the `alloc` crate under an alias to avoid conflict with `mod alloc` above.
// Exposed as `crate::alloc_crate` so submodules can use `use crate::alloc_crate::vec::Vec;`
#[cfg(zisk_guest)]
extern crate alloc as alloc_crate;
#[cfg(zisk_guest)]
pub(crate) use alloc_crate as alloc_extern;

mod profile;
#[cfg(zisk_guest)]
pub use fcall::*;
pub mod io;
pub use profile::*;
pub mod syscalls;
pub mod zisklib;
pub mod ziskos_definitions;

#[cfg(all(not(zisk_guest), any(zisk_hints, zisk_hints_debug), feature = "user-hints"))]
pub mod hints;

#[cfg(all(not(zisk_guest), zisk_hints))]
extern "C" {
    fn hint_input_data(input_data_ptr: *const u8, input_data_len: usize);
}

#[cfg(all(not(zisk_guest), zisk_hints_debug))]
extern "C" {
    fn hint_log_c(msg: *const std::os::raw::c_char);
}

#[cfg(zisk_hints_debug)]
pub fn hint_log<S: AsRef<str>>(msg: S) {
    // On native we call external C function to log hints, since it controls if hints are paused or not
    #[cfg(not(zisk_guest))]
    {
        use std::ffi::CString;

        if let Ok(c) = CString::new(msg.as_ref()) {
            unsafe { hint_log_c(c.as_ptr()) };
        }
    }
    // On zkvm/zisk, we can just print directly
    #[cfg(zisk_guest)]
    {
        println!("{}", msg.as_ref());
    }
}

#[cfg_attr(all(not(feature = "hints"), not(zisk_staticlib)), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_init")]
pub extern "C" fn zkvm_init() {
    #[cfg(not(zisk_guest))]
    {
        read_input_reset();
        crate::zisklib::zkvm_io::reset();
    }

    #[cfg(all(not(zisk_guest), zisk_hints, feature = "user-hints"))]
    {
        let path =
            std::env::var("ZISK_HINTS_OUTPUT").map(std::path::PathBuf::from).unwrap_or_else(|_| {
                let dir = std::path::PathBuf::from("./tmp");
                std::fs::create_dir_all(&dir).expect("failed to create tmp dir");
                dir.join("hints.bin")
            });
        crate::hints::init_hints_file(path, None).expect("hints init failed");
    }
}

#[cfg_attr(all(not(feature = "hints"), not(zisk_staticlib)), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_deinit")]
pub extern "C" fn zkvm_deinit() {
    #[cfg(all(not(zisk_guest), zisk_hints, feature = "user-hints"))]
    {
        crate::hints::close_hints().expect("hints close failed");
    }

    // End of the staticlib lifecycle: report the isolated allocator's peak usage.
    #[cfg(all(zisk_guest, zisk_staticlib, feature = "alloc-stats"))]
    unsafe {
        crate::alloc::print_max_used_sys_alloc();
    }
}

#[cfg(all(not(zisk_guest), zisk_hints, feature = "user-hints"))]
pub fn zkvm_init_socket(
    socket_path: std::path::PathBuf,
    debug_file: Option<std::path::PathBuf>,
    write_flush_threshold: Option<usize>,
    ready: Option<tokio::sync::oneshot::Sender<()>>,
) -> anyhow::Result<()> {
    read_input_reset();
    crate::zisklib::zkvm_io::reset();
    crate::hints::init_hints_socket(socket_path, debug_file, write_flush_threshold, ready)
}

#[macro_export]
macro_rules! entrypoint {
    ($path:path) => {
        const ZISK_ENTRY: fn() = $path;

        mod zkvm_generated_main {
            // C ABI to match the `extern "C" { fn main() -> i32; }` declaration in
            // `_zisk_main` that calls this symbol — mixing ABIs on the same symbol
            // is undefined behavior.
            #[no_mangle]
            extern "C" fn main() -> i32 {
                $crate::zkvm_init();
                super::ZISK_ENTRY();
                $crate::zkvm_deinit();
                // Guest entry returns `()`, so a normal completion is success.
                0
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
#[cfg(zisk_guest)]
pub(crate) const INPUT_INITIAL_OFFSET: usize = 8;
#[cfg(not(zisk_guest))]
pub(crate) const INPUT_INITIAL_OFFSET: usize = 0;

/// Pointer to the current position in the input buffer/file.
pub(crate) static mut INPUT_POS: usize = INPUT_INITIAL_OFFSET;

/// Reset the input position to the beginning.
pub fn read_input_reset() {
    unsafe { INPUT_POS = INPUT_INITIAL_OFFSET };
}

#[cfg(not(zisk_guest))]
static NATIVE_INPUT: std::sync::Mutex<Option<Vec<u8>>> = std::sync::Mutex::new(None);

#[cfg(not(zisk_guest))]
pub fn set_native_input(data: Vec<u8>) {
    *NATIVE_INPUT.lock().unwrap() = Some(data);
}

/// Read a slice directly from INPUT_ADDR without copying (zero-copy).
///
/// This returns a slice pointing directly to the input memory region.
/// Use this when you want to deserialize directly without an intermediate copy.
/// The INPUT_POS is advanced after this call.
#[cfg(zisk_guest)]
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

#[cfg(not(zisk_guest))]
pub(crate) fn read_input() -> Vec<u8> {
    let input_pos = unsafe { INPUT_POS };

    let data = if let Some(buf) = NATIVE_INPUT.lock().unwrap().as_ref() {
        let len_bytes: [u8; 8] = buf
            .get(input_pos..input_pos + 8)
            .expect("Failed to read length prefix from native input")
            .try_into()
            .unwrap();
        let len = u64::from_le_bytes(len_bytes) as usize;
        let data = buf
            .get(input_pos + 8..input_pos + 8 + len)
            .expect("Failed to read data from native input")
            .to_vec();
        let aligned_len = (len + 7) & !0x7;
        unsafe { INPUT_POS = input_pos + 8 + aligned_len };
        data
    } else {
        use std::{
            fs::File,
            io::{Read, Seek, SeekFrom},
        };

        let path =
            std::env::var("ZISK_INPUT_FILE").unwrap_or_else(|_| "build/input.bin".to_string());

        let mut file = File::open(&path)
            .unwrap_or_else(|e| panic!("Error opening input file at {}: {}", path, e));

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
        unsafe { INPUT_POS = input_pos + 8 + aligned_len };

        data
    };

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

#[cfg(zisk_guest)]
pub(crate) fn set_output(id: usize, value: u32) {
    use core::arch::asm;
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

#[cfg(not(zisk_guest))]
pub(crate) fn set_output(id: usize, value: u32) {
    println!("public {id}: {value:#010x}");
}

#[cfg(zisk_guest)]
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

          // Call into Rust. `_zisk_main` returns `main`'s exit code in a0,
          // which both exit paths below forward to the termination mechanism.
          "call {_zisk_main}",
          "csrr t0, marchid",
          //"li   t1, {_ARCH_ID_ZISK}",
          "li   t1, 0xFFFEEEE",
          "beq t0, t1, 1f",

          // QEMU exit via the sifive_test device @ 0x100000. Encode the exit
          // code (a0): 0 => 0x5555 (pass); otherwise (code << 16) | 0x3333.
          "li t0, 0x100000",
          "beqz a0, 3f",
          "slli t1, a0, 16",
          "li   t2, 0x3333",
          "or   t1, t1, t2",
          "sw t1, 0(t0)",
          "j 2f",
          "3:",
          "li t1, 0x5555",
          "sw t1, 0(t0)",
          "j 2f",

          // Zisk exit: syscall 93 (exit) takes the exit code in a0, already set
          // by `_zisk_main`'s return value.
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
    unsafe extern "C" fn _zisk_main() -> i32 {
        {
            extern "C" {
                // Standard `int main(void)`: its return value is the program
                // exit code, propagated to the termination mechanism below.
                fn main() -> i32;
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

            // Run C++ (and Rust ctor-style) static constructors before main.
            run_init_array();

            let code = main();

            // Run static destructors after main returns (atexit-registered first,
            // then `.fini_array`), before `_start` invokes the exit mechanism.
            run_exit_handlers();

            code
        }
    }

    // ---- C++ static constructors / destructors (zkvm-standards) -------------
    //
    // Run `.init_array` before `main` and the destructors after it returns.
    // Empty arrays => no-ops, so this is zero-cost for programs without C++ (or
    // Rust ctor-style) statics. Allocations done by constructors resolve to the
    // host program's allocator, NOT ziskos's private bump heap (whose symbols are
    // localized and which is rewound per call) — exactly what persistent statics
    // need.

    type CtorFn = extern "C" fn();

    // Linker-provided bounds of the constructor/destructor arrays, defined by the
    // linker script (see linker_script.ld). Declared as zero-length arrays with
    // *normal* (strong) linkage so that `addr_of!` yields the section address
    // directly. (An `extern_weak` static is reached through a hidden pointer slot,
    // so `addr_of!` would not give the array address — it must be a plain symbol.)
    extern "C" {
        static __init_array_start: [CtorFn; 0];
        static __init_array_end: [CtorFn; 0];
        static __fini_array_start: [CtorFn; 0];
        static __fini_array_end: [CtorFn; 0];
    }

    unsafe fn run_init_array() {
        let mut p = core::ptr::addr_of!(__init_array_start) as *const CtorFn;
        let end = core::ptr::addr_of!(__init_array_end) as *const CtorFn;
        // Empty array (no C++/ctor statics) => p == end => no iterations.
        while p < end {
            (core::ptr::read(p))();
            p = p.add(1);
        }
    }

    unsafe fn run_fini_array() {
        let start = core::ptr::addr_of!(__fini_array_start) as *const CtorFn;
        let mut p = core::ptr::addr_of!(__fini_array_end) as *const CtorFn;
        // `.fini_array` runs in reverse order; empty => no iterations.
        while p > start {
            p = p.sub(1);
            (core::ptr::read(p))();
        }
    }

    // Minimal freestanding C++ exit machinery. The compiler registers static
    // destructors at construction time via `__cxa_atexit(dtor, obj, &__dso_handle)`
    // and expects them to run at exit via `__cxa_finalize`. Single-threaded guest,
    // so a plain fixed-capacity `static mut` table suffices. Weak so a host C++
    // runtime (e.g. libsupc++), if linked, takes precedence.

    #[no_mangle]
    #[linkage = "weak"]
    pub static __dso_handle: u8 = 0;

    const MAX_ATEXIT: usize = 64;
    static mut ATEXIT_FNS: [(Option<extern "C" fn(*mut u8)>, *mut u8); MAX_ATEXIT] =
        [(None, core::ptr::null_mut()); MAX_ATEXIT];
    static mut ATEXIT_LEN: usize = 0;

    #[no_mangle]
    #[linkage = "weak"]
    pub unsafe extern "C" fn __cxa_atexit(
        func: extern "C" fn(*mut u8),
        arg: *mut u8,
        _dso: *mut u8,
    ) -> i32 {
        if ATEXIT_LEN >= MAX_ATEXIT {
            return -1;
        }
        // Raw-pointer write to avoid taking a reference to the `static mut`.
        let base =
            core::ptr::addr_of_mut!(ATEXIT_FNS) as *mut (Option<extern "C" fn(*mut u8)>, *mut u8);
        core::ptr::write(base.add(ATEXIT_LEN), (Some(func), arg));
        ATEXIT_LEN += 1;
        0
    }

    #[no_mangle]
    #[linkage = "weak"]
    pub unsafe extern "C" fn __cxa_finalize(_dso: *mut u8) {
        // Reverse registration order; idempotent (drains the table), so a second
        // call (e.g. from `.fini_array`) is a no-op.
        let base =
            core::ptr::addr_of!(ATEXIT_FNS) as *const (Option<extern "C" fn(*mut u8)>, *mut u8);
        while ATEXIT_LEN > 0 {
            ATEXIT_LEN -= 1;
            let (func, arg) = core::ptr::read(base.add(ATEXIT_LEN));
            if let Some(func) = func {
                func(arg);
            }
        }
    }

    unsafe fn run_exit_handlers() {
        __cxa_finalize(core::ptr::null_mut());
        run_fini_array();
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
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};

    // riscv64 guest targets are single-core — sys_rand uses static mut, no atomics needed
    static mut RNG: Option<SmallRng> = None;
    static mut SYS_RAND_WARNING: bool = false;

    #[allow(static_mut_refs)]
    #[no_mangle]
    unsafe extern "C" fn sys_rand(recv_buf: *mut u8, words: usize) {
        if !SYS_RAND_WARNING {
            SYS_RAND_WARNING = true;
            let msg = b"WARNING: Using insecure random number generator.\n";
            sys_write(1, msg.as_ptr(), msg.len());
        }
        let rng = RNG.get_or_insert_with(|| SmallRng::seed_from_u64(0x123456789abcdef0));
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

    pub extern "C" fn sys_write_hex(val: usize, ln: bool) {
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

    pub extern "C" fn sys_write_u64(val: u64, ln: bool) {
        let mut buf = [0u8; 21]; // 20 digits max u64 + \n — stack, no heap
        let mut v = val;
        let mut end = 20usize;

        if v == 0 {
            buf[19] = b'0';
            end = 19;
        } else {
            while v > 0 {
                end -= 1;
                buf[end] = b'0' + (v % 10) as u8;
                v /= 10;
            }
        }

        if ln {
            buf[20] = b'\n';
            sys_write(1, buf[end..].as_ptr(), 21 - end);
        } else {
            sys_write(1, buf[end..20].as_ptr(), 20 - end);
        }
    }

    core::arch::global_asm!(include_str!("dma/memcpy.s"));
    core::arch::global_asm!(include_str!("dma/memmove.s"));
    core::arch::global_asm!(include_str!("dma/memcmp.s"));
    //core::arch::global_asm!(include_str!("dma/inputcpy.s"));
    core::arch::global_asm!(include_str!("dma/memset.s"));
}
