//! Memory accesses in the part of the RAM above 4 GB (byte addresses with bit 32 set) and across
//! the 2^32 boundary: misaligned and one-byte loads and stores (the MemAlign airs), keccak-f states
//! and DMA copies (block writes) placed there. Every round repeats them, so the block writes land
//! at every position of the emulator's chunks. The memory witness built on the device must equal
//! the CPU one on this program.
//!
//! Input: the number of rounds (u64). Public output: a digest of what was read back (u64 LE).

#![no_main]
ziskos::entrypoint!(main);

use core::arch::asm;
use ziskos::syscalls::syscall_keccak_f;

/// The first byte above 4 GB.
const ABOVE_4GB: u64 = 1 << 32;
/// One past the last byte of the RAM (core/src/mem.rs).
const RAM_END: u64 = 0x1_4000_0000;

fn main() {
    let rounds: u64 = ziskos::io::read();
    let mut src = [0u8; 320];
    let mut digest = 0u64;
    for r in 0..rounds {
        let x = r.wrapping_add(1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        digest = digest.rotate_left(7) ^ mem_align_round(x);
        digest = digest.rotate_left(7) ^ keccak_round(x);
        digest = digest.rotate_left(7) ^ dma_round(&mut src, x);
    }
    ziskos::io::commit_slice(&digest.to_le_bytes());
}

// The loads and stores are written in assembly so that each one is a single instruction at the
// given (misaligned) address, whatever the compiler would make of a Rust access.
macro_rules! store {
    ($op:literal, $addr:expr, $value:expr) => {
        // SAFETY: every address passed is inside the RAM (core/src/mem.rs) and above the heap of
        // this program, which never grows near it; nothing else reads or writes these bytes.
        unsafe { asm!(concat!($op, " {v}, 0({a})"), a = in(reg) $addr, v = in(reg) $value, options(nostack)) }
    };
}
macro_rules! load {
    ($op:literal, $addr:expr) => {{
        let v: u64;
        // SAFETY: as for `store!`.
        unsafe { asm!(concat!($op, " {v}, 0({a})"), a = in(reg) $addr, v = out(reg) v, options(nostack, readonly)) }
        v
    }};
}

/// One store and one load of every width at misaligned addresses above 4 GB, across 2^32 and at
/// the top of the RAM.
fn mem_align_round(x: u64) -> u64 {
    let base = ABOVE_4GB + 0x100 + (x & 0xFF8);
    store!("sb", base + 1, x);
    store!("sh", base + 3, x >> 8);
    store!("sw", base + 13, x >> 16); // bytes 13-16: two words
    store!("sd", base + 21, x); // bytes 21-28: two words
    store!("sd", ABOVE_4GB - 3, x >> 3); // one word below 2^32, one above
    store!("sb", RAM_END - 1, x >> 5);
    store!("sw", RAM_END - 6, x >> 7);
    store!("sd", RAM_END - 11, x >> 9); // the last two words of the RAM
    let mut d = load!("lbu", base + 1);
    d = d.rotate_left(9) ^ load!("lh", base + 3);
    d = d.rotate_left(9) ^ load!("lw", base + 13);
    d = d.rotate_left(9) ^ load!("ld", base + 21);
    d = d.rotate_left(9) ^ load!("ld", ABOVE_4GB - 3);
    d = d.rotate_left(9) ^ load!("lw", ABOVE_4GB - 2);
    d = d.rotate_left(9) ^ load!("lbu", RAM_END - 1);
    d = d.rotate_left(9) ^ load!("lw", RAM_END - 6);
    d.rotate_left(9) ^ load!("ld", RAM_END - 11)
}

/// keccak-f on a state above 4 GB and on one that crosses 2^32 (96 bytes below, 104 above).
fn keccak_round(x: u64) -> u64 {
    let mut d = 0;
    for st in [ABOVE_4GB + 0x2000, ABOVE_4GB - 96] {
        let st = st as *mut [u64; 25];
        // SAFETY: the state is 8-byte aligned, inside the RAM and used by nothing else (see
        // `store!`); the precompile reads and writes its 25 words.
        unsafe {
            (*st)[0] ^= x;
            syscall_keccak_f(st);
            d ^= (*st)[0] ^ (*st)[24];
        }
    }
    d
}

/// A DMA copy to a misaligned address above 4 GB, and back, of a length that changes every round.
fn dma_round(src: &mut [u8; 320], x: u64) -> u64 {
    src[(x % 320) as usize] = x as u8;
    let len = 256 + (x & 63) as usize;
    let dst = (ABOVE_4GB + 0x3003) as *mut u8;
    let mut back = [0u8; 320];
    // SAFETY: `dst..dst + len` is inside the RAM and used by nothing else (see `store!`); `src`
    // and `back` are local buffers of at least `len` bytes that do not overlap it.
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), dst, len);
        core::ptr::copy_nonoverlapping(dst.add(len / 3), back.as_mut_ptr(), len - len / 3);
    }
    u64::from_le_bytes(back[..8].try_into().unwrap()) ^ back[len - len / 3 - 1] as u64
}
