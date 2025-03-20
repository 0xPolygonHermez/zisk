#![allow(unexpected_cfgs)]

pub mod syscalls;

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

#[allow(unused_imports)]
use crate::ziskos_definitions::ziskos_config::*;

#[cfg(target_os = "ziskos")]
pub fn read_input() -> Vec<u8> {
    // Create a slice of the first 8 bytes to get the size
    let bytes = unsafe { core::slice::from_raw_parts(INPUT_ADDR as *const u8, 8) };
    // Convert the slice to a u64 (little-endian)
    let size: u64 = u64::from_le_bytes(bytes.try_into().unwrap());

    let input =
        unsafe { core::slice::from_raw_parts((INPUT_ADDR as *const u8).add(8), size as usize) };
    input.to_vec()
}

#[cfg(not(target_os = "ziskos"))]
pub fn read_input() -> Vec<u8> {
    use std::{fs::File, io::Read};

    let mut file =
        File::open("build/input.bin").expect("Error opening input file at: build/input.bin");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    buffer
}

#[cfg(target_os = "ziskos")]
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

#[cfg(not(target_os = "ziskos"))]
pub fn set_output(id: usize, value: u32) {
    println!("public {}: {:#010x}", id, value);
}

#[cfg(target_os = "ziskos")]
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
            addr = 0xa000_0200 as *mut u8;
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
}
