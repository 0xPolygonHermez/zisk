//! Smoke guest: calls the precompile N times on a guarded state and commits the result.
//! Input byte 0 selects the case: 0/1/8/9, 0xfc = 1024, 0xfd = 1025 calls; 0x80 + lane
//! writes a noncanonical lane; 0xa0 + offset misaligns the pointer. Both must abort.
#![no_main]

ziskos::entrypoint!(main);
include!(concat!(env!("OUT_DIR"), "/expected.rs"));

#[repr(C, align(8))]
struct GuardedState {
    before: u64,
    state: [u64; 8],
    after: u64,
}

unsafe fn raw_syscall(state: *mut [u64; 8]) {
    #[cfg(target_arch = "riscv64")]
    core::arch::asm!(
        "csrs {syscall}, {state}",
        syscall = const zisk_definitions::SYSCALL_KOALA_POSEIDON2_ID,
        state = in(reg) state,
        options(nostack),
    );
    #[cfg(not(target_arch = "riscv64"))]
    ziskos::syscalls::syscall_koala_poseidon2(state);
}

pub fn main() {
    let input = ziskos::io::read_slice();
    assert_eq!(input.len(), 1);
    let calls = input[0];
    let mut guarded = GuardedState {
        before: 0x0123_4567_89ab_cdef,
        state: EXPECTED[0],
        after: 0xfedc_ba98_7654_3210,
    };
    if (0x80..0x90).contains(&calls) {
        let lane = usize::from(calls - 0x80);
        let shift = 32 * (lane % 2);
        guarded.state[lane / 2] &= !(u64::from(u32::MAX) << shift);
        guarded.state[lane / 2] |= u64::from(zisk_definitions::koala_poseidon2::MODULUS) << shift;
        unsafe { raw_syscall(&mut guarded.state) };
        panic!("noncanonical input was accepted");
    }
    if (0xa1..0xa8).contains(&calls) {
        let pointer = guarded.state.as_mut_ptr().cast::<u8>();
        let pointer = unsafe { pointer.add(usize::from(calls - 0xa0)) }.cast::<[u64; 8]>();
        unsafe { raw_syscall(pointer) };
        panic!("misaligned input was accepted");
    }
    let calls: usize = match calls {
        0 | 1 | 8 | 9 => usize::from(calls),
        0xfc => 1024,
        0xfd => 1025,
        _ => panic!("unknown smoke case"),
    };
    for _ in 0..calls {
        unsafe { ziskos::syscalls::syscall_koala_poseidon2(&mut guarded.state) };
    }
    assert_eq!(guarded.state, EXPECTED[calls]);
    assert_eq!(guarded.before, 0x0123_4567_89ab_cdef);
    assert_eq!(guarded.after, 0xfedc_ba98_7654_3210);
    let mut output = [0_u8; 68];
    output[..4].copy_from_slice(&(calls as u32).to_le_bytes());
    for (word, bytes) in guarded.state.iter().zip(output[4..].chunks_exact_mut(8)) {
        bytes.copy_from_slice(&word.to_le_bytes());
    }
    ziskos::io::commit_slice(&output);
}
