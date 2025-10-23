#![allow(unexpected_cfgs)]
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod fcall;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub use fcall::*;

pub mod zisklib;
pub use zisklib::*;

mod syscalls;
pub use syscalls::*;

pub mod ziskos_definitions;

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

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub fn read_input() -> Vec<u8> {
    read_input_slice().to_vec()
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
pub fn read_input() -> Vec<u8> {
    use std::{fs::File, io::Read};

    let mut file =
        File::open("build/input.bin").expect("Error opening input file at: build/input.bin");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    buffer
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub fn read_input_slice<'a>() -> &'a [u8] {
    // Create a slice of the first 8 bytes to get the size
    let bytes = unsafe { core::slice::from_raw_parts((INPUT_ADDR as *const u8).add(8), 8) };
    // Convert the slice to a u64 (little-endian)
    let size: u64 = u64::from_le_bytes(bytes.try_into().unwrap());

    unsafe { core::slice::from_raw_parts((INPUT_ADDR as *const u8).add(16), size as usize) }
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
pub fn read_input_slice() -> Box<[u8]> {
    read_input().into_boxed_slice()
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub fn set_output(id: usize, value: u32) {
    use std::arch::asm;
    let addr_n: *mut u32;
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
        addr_n = OUTPUT_ADDR as *mut u32;
        addr_v = (OUTPUT_ADDR + 4 + 4 * (id as u64)) as *mut u32;
    } else {
        addr_n = 0x1000_0000 as *mut u32;
        addr_v = (0x1000_0000 + 4 + 4 * (id as u64)) as *mut u32;
    }

    let n;

    unsafe {
        n = core::ptr::read(addr_n) as usize;
    }

    if id + 1 > n {
        unsafe { core::ptr::write_volatile(addr_n, (id + 1) as u32) };
    }

    unsafe { core::ptr::write_volatile(addr_v, value) };
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
pub fn set_output(id: usize, value: u32) {
    println!("public {id}: {value:#010x}");
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod ziskos {
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
            main()
        }
    }

    #[no_mangle]
    extern "C" fn sys_write(_fd: u32, write_ptr: *const u8, nbytes: usize) {
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

    #[no_mangle]
    pub unsafe extern "C" fn sys_alloc_aligned(bytes: usize, align: usize) -> *mut u8 {
        use core::arch::asm;
        let heap_bottom: usize;
        // UNSAFE: This is fine, just loading some constants.
        unsafe {
            // using inline assembly is easier to access linker constants
            asm!(
              "la {heap_bottom}, _kernel_heap_bottom",
              heap_bottom = out(reg) heap_bottom,
              options(nomem)
            )
        };

        // Pointer to next heap address to use, or 0 if the heap has not yet been
        // initialized.
        static mut HEAP_POS: usize = 0;

        // SAFETY: Single threaded, so nothing else can touch this while we're working.
        let mut heap_pos = unsafe { HEAP_POS };

        if heap_pos == 0 {
            heap_pos = heap_bottom;
        }

        let offset = heap_pos & (align - 1);
        if offset != 0 {
            heap_pos += align - offset;
        }

        let ptr = heap_pos as *mut u8;
        heap_pos += bytes;

        // Check to make sure heap doesn't collide with SYSTEM memory.
        //if SYSTEM_START < heap_pos {
        //    panic!();
        // }

        unsafe { HEAP_POS = heap_pos };

        ptr
    }
    /// 64-bit optimized memcpy for RISC-V - extern "C" version
    ///
    /// # Safety
    /// This function is unsafe because it works with raw pointers and does not
    /// perform bounds checking or pointer validity verification.
    ///
    /// # Arguments
    /// * `dst` - Destination pointer
    /// * `src` - Source pointer  
    /// * `len` - Number of bytes to copy
    ///
    /// # Returns
    /// Returns the original destination pointer
    #[no_mangle]
    pub extern "C" fn memcpy(dst: *mut u8, src: *const u8, len: usize) -> *mut u8 {
        unsafe {
            asm!(
                // Initialize working pointers
                "mv     {src_work}, {src_input}",    // src_work = src
                "mv     {dst_work}, {dst_input}",    // dst_work = dst

                // Check if len is 0
                "beqz   {len_work}, 25f",            // If len=0, jump to end

                // Check if src is aligned to 8 bytes
                "andi   {tmp1}, {src_work}, 7",      // tmp1 = src & 7
                "beqz   {tmp1}, 8f",                 // If aligned, jump to .L8

                // Alignment loop for src (byte by byte until aligned)
                "2:",                                // .L2 (alignment loop)
                "lb     {tmp2}, 0({src_work})",      // Load byte from src
                "sb     {tmp2}, 0({dst_work})",      // Store byte to dst
                "addi   {src_work}, {src_work}, 1",  // src++
                "addi   {dst_work}, {dst_work}, 1",  // dst++
                "addi   {len_work}, {len_work}, -1", // len--
                "beqz   {len_work}, 25f",            // If len=0, finish
                "andi   {tmp1}, {src_work}, 7",      // Check src alignment
                "bnez   {tmp1}, 2b",                 // If not aligned, continue loop

                "8:",                                // .L8 (src is now aligned)
                // Check dst alignment
                "andi   {tmp1}, {dst_work}, 7",      // Check dst alignment
                "bnez   {tmp1}, 4f",                 // If dst not aligned, use 32-bit

                // Both pointers aligned to 8 bytes
                "9:",                                // .L9 (both pointers aligned to 8 bytes)
                "li     {tmp1}, 32",                 // Threshold for 32-byte loop
                "bltu   {len_work}, {tmp1}, 12f",    // If len < 32, jump to .L12

                // Main 64-bit loop (32 bytes per iteration)
                "11:",                               // .L11 (main 64-bit loop)
                "ld     {tmp1}, 0({src_work})",      // Load 8 bytes
                "ld     {tmp2}, 8({src_work})",      // Load 8 bytes
                "ld     {tmp3}, 16({src_work})",     // Load 8 bytes
                "ld     {tmp4}, 24({src_work})",     // Load 8 bytes
                "sd     {tmp1}, 0({dst_work})",      // Store 8 bytes
                "sd     {tmp2}, 8({dst_work})",      // Store 8 bytes
                "sd     {tmp3}, 16({dst_work})",     // Store 8 bytes
                "sd     {tmp4}, 24({dst_work})",     // Store 8 bytes
                "addi   {src_work}, {src_work}, 32", // src += 32
                "addi   {dst_work}, {dst_work}, 32", // dst += 32
                "addi   {len_work}, {len_work}, -32", // len -= 32
                "li     {tmp1}, 31",                 // tmp1 = 31 (for comparison)
                "bltu   {tmp1}, {len_work}, 11b",    // If len > 31, continue loop

                "12:",                               // .L12 (process 16-byte blocks)
                "andi   {tmp1}, {len_work}, 16",     // len & 16
                "beqz   {tmp1}, 14f",                // If no 16 bytes, skip

                "ld     {tmp1}, 0({src_work})",      // Load 8 bytes
                "ld     {tmp2}, 8({src_work})",      // Load 8 bytes
                "sd     {tmp1}, 0({dst_work})",      // Store 8 bytes
                "sd     {tmp2}, 8({dst_work})",      // Store 8 bytes
                "addi   {src_work}, {src_work}, 16", // src += 16
                "addi   {dst_work}, {dst_work}, 16", // dst += 16

                "14:",                               // .L14 (process 8-byte blocks)
                "andi   {tmp1}, {len_work}, 8",      // len & 8
                "beqz   {tmp1}, 22f",                // If no 8 bytes, skip

                "ld     {tmp1}, 0({src_work})",      // Load 8 bytes
                "sd     {tmp1}, 0({dst_work})",      // Store 8 bytes
                "addi   {src_work}, {src_work}, 8",  // src += 8
                "addi   {dst_work}, {dst_work}, 8",  // dst += 8
                "j      22f",                        // Go to process remaining bytes

                "4:",                                // .L4 (32-bit fallback - dst unaligned)
                // Main 32-bit loop (16 bytes per iteration)
                "li     {tmp1}, 16",                 // Threshold for 16-byte loop
                "bltu   {len_work}, {tmp1}, 20f",    // If len < 16, jump to .L20

                "19:",                               // .L19 (32-bit loop)
                "lw     {tmp1}, 0({src_work})",      // Load 4 bytes
                "lw     {tmp2}, 4({src_work})",      // Load 4 bytes
                "lw     {tmp3}, 8({src_work})",      // Load 4 bytes
                "lw     {tmp4}, 12({src_work})",     // Load 4 bytes
                "sw     {tmp1}, 0({dst_work})",      // Store 4 bytes
                "sw     {tmp2}, 4({dst_work})",      // Store 4 bytes
                "sw     {tmp3}, 8({dst_work})",      // Store 4 bytes
                "sw     {tmp4}, 12({dst_work})",     // Store 4 bytes
                "addi   {src_work}, {src_work}, 16", // src += 16
                "addi   {dst_work}, {dst_work}, 16", // dst += 16
                "addi   {len_work}, {len_work}, -16", // len -= 16
                "li     {tmp1}, 15",                 // tmp1 = 15 (for comparison)
                "bltu   {tmp1}, {len_work}, 19b",    // If len > 15, continue loop

                "20:",                               // .L20 (process 8-byte block with 32-bit)
                "andi   {tmp1}, {len_work}, 8",      // len & 8
                "beqz   {tmp1}, 22f",                // If no 8 bytes, skip

                "lw     {tmp1}, 0({src_work})",      // Load 4 bytes
                "lw     {tmp2}, 4({src_work})",      // Load 4 bytes
                "sw     {tmp1}, 0({dst_work})",      // Store 4 bytes
                "sw     {tmp2}, 4({dst_work})",      // Store 4 bytes
                "addi   {src_work}, {src_work}, 8",  // src += 8
                "addi   {dst_work}, {dst_work}, 8",  // dst += 8

                "22:",                               // .L22 (process remaining bytes)
                "andi   {tmp1}, {len_work}, 4",      // len & 4
                "beqz   {tmp1}, 23f",                // If no 4 bytes, skip

                "lw     {tmp1}, 0({src_work})",      // Load 4 bytes
                "sw     {tmp1}, 0({dst_work})",      // Store 4 bytes
                "addi   {src_work}, {src_work}, 4",  // src += 4
                "addi   {dst_work}, {dst_work}, 4",  // dst += 4

                "23:",                               // .L23 (process 2 bytes)
                "andi   {tmp1}, {len_work}, 2",      // len & 2
                "beqz   {tmp1}, 24f",                // If no 2 bytes, skip

                "lh     {tmp1}, 0({src_work})",      // Load 2 bytes
                "sh     {tmp1}, 0({dst_work})",      // Store 2 bytes
                "addi   {src_work}, {src_work}, 2",  // src += 2
                "addi   {dst_work}, {dst_work}, 2",  // dst += 2

                "24:",                               // .L24 (process last byte)
                "andi   {tmp1}, {len_work}, 1",      // len & 1
                "beqz   {tmp1}, 25f",                // If no remaining byte, finish

                "lb     {tmp1}, 0({src_work})",      // Load last byte
                "sb     {tmp1}, 0({dst_work})",      // Store last byte

                "25:",                               // .L25 (end)

                // Outputs (working registers)
                src_work = out(reg) _,          // Working pointer for src
                dst_work = out(reg) _,          // Working pointer for dst
                len_work = inout(reg) len => _, // len is modified during execution
                tmp1 = out(reg) _,              // Temporaries
                tmp2 = out(reg) _,
                tmp3 = out(reg) _,
                tmp4 = out(reg) _,

                // Inputs (read-only)
                dst_input = in(reg) dst,
                src_input = in(reg) src,

                options(nostack, preserves_flags)
            );
        }

        dst
    }
}
