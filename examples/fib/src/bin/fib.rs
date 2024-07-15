#![feature(naked_functions)]
#![feature(asm_const)]

#[cfg(target_os = "ziskos")]
use core::arch::asm;
use prost::Message;
use std::fs::File;
use std::io::{self, Read};

const MAX_INPUT: usize = 0x2000;
const MAX_OUTPUT: usize = 0x1_0000;

#[cfg(target_os = "ziskos")]
mod ziskos_config {
    pub static mut SV: u64 = 0xBBBB;

    pub const QEMU_EXIT_ADDR: u64 = 0x100000;
    pub const QEMU_EXIT_CODE: u64 = 0x5555;
    pub const INPUT_ADDR: u64 = 0x9000_0000;
    pub const OUTPUT_ADDR: u64 = 0xa001_0000;
    pub const ARCH_ID_ZISK: u64 = 0xFFFEEEE; // TEMPORARY  // TODO register one
}
mod input {
    include!(concat!(env!("OUT_DIR"), "/inputs.rs"));
}

#[cfg(target_os = "ziskos")]
use input::Input;

#[cfg(target_os = "ziskos")]
use ziskos_config::*;

#[naked]
#[no_mangle]
#[link_section = ".text.init"]
#[cfg(target_os = "ziskos")]
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
      "li   t1, {ARCH_ID_ZISK}",
      "beq t0, t1, 1f",

      // QEmuu exit
      "li t0, {QEMU_EXIT_ADDR}",
      "li t1, {QEMU_EXIT_CODE}",
      "sw t1, 0(t0)",
      "j 2f",

      // Zisk exit
      "1: li   a7, 93",
      "ecall",

      "2: j 2b",

      _zisk_main = sym _zisk_main, // {entry} refers to the function [entry] below
      QEMU_EXIT_ADDR = const QEMU_EXIT_ADDR,
      QEMU_EXIT_CODE = const QEMU_EXIT_CODE,
      ARCH_ID_ZISK = const ARCH_ID_ZISK,
      options(noreturn) // we must handle "returning" from assembly
    );
}

// Function to get the size of the serialized message
unsafe fn get_serialized_size(input: *const u8) -> u32 {
    let size_bytes = core::slice::from_raw_parts(input, 4); // Read the first 4 bytes
    u32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]])
}

#[cfg(target_os = "ziskos")]
#[no_mangle]
extern "C" fn _zisk_main() {
    //check if static value works
    unsafe { SV += 1 };

    #[cfg(target_os = "ziskos")]
    {
        // Create a slice of the first 4 bytes
        let bytes = unsafe { core::slice::from_raw_parts(INPUT_ADDR as *const u8, 8) };
        // Convert the slice to a u64 (little-endian)
        let size = usize::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);

        println!("Input file size: {} bytes", size);

        let input =
            unsafe { core::slice::from_raw_parts((INPUT_ADDR as *const u8).add(8), size) };

        let output = unsafe { core::slice::from_raw_parts_mut(OUTPUT_ADDR as *mut _, MAX_OUTPUT) };

        fib::main(input, output);
    }
}
const FILENAME: &str = "input.bin";
fn main() -> io::Result<()> {
    let mut file = File::open(FILENAME)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let mut output: Vec<u8> = vec![0; MAX_OUTPUT];

    // Create a slice of the first 4 bytes
    let bytes = unsafe { core::slice::from_raw_parts(buf.as_ptr(), 8) };
    // Convert the slice to a u32 (little-endian)
    let size = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

    let input = unsafe { core::slice::from_raw_parts(buf.as_ptr(), buf.len()) };

    fib::main(input, &mut output);
    Ok(())
}

#[no_mangle]
#[cfg(target_os = "ziskos")]
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

#[no_mangle]
#[cfg(target_os = "ziskos")]
extern "C" fn sys_getenv() {
    //unimplemented!("sys_getenv")
}

#[no_mangle]
#[cfg(target_os = "ziskos")]

extern "C" fn sys_alloc_words() {
    //unimplemented!("sys_alloc_words")
}

#[no_mangle]
#[cfg(target_os = "ziskos")]
extern "C" fn sys_argc() {
    unimplemented!("sys_argc");
}

#[no_mangle]
#[cfg(target_os = "ziskos")]
extern "C" fn sys_argv() {
    unimplemented!("sys_argv");
}

#[no_mangle]
#[cfg(target_os = "ziskos")]
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
