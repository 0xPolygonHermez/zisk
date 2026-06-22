// Proof-of-concept guest for the `babyjubjub_add` precompile.
// Reads two affine BabyJubJub (twisted Edwards) points from the input stream,
// each coordinate a 256-bit BN254 scalar-field element encoded as 32 little-endian
// bytes (x then y), computes `p1 + p2` via the `syscall_babyjubjub_add` precompile,
// and commits the resulting point `x3 || y3` (64 little-endian bytes).

#![no_main]
ziskos::entrypoint!(main);

use ziskos::syscalls::{syscall_babyjubjub_add, SyscallBabyJubJubAddParams, SyscallPoint256};

fn read_coord(bytes: &[u8]) -> [u64; 4] {
    let mut c = [0u64; 4];
    for (i, limb) in c.iter_mut().enumerate() {
        *limb = u64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().unwrap());
    }
    c
}

fn main() {
    let input = ziskos::io::read_input_slice();
    assert!(input.len() >= 128, "expected at least 128 bytes (two points), got {}", input.len());

    let mut p1 = SyscallPoint256 { x: read_coord(&input[0..32]), y: read_coord(&input[32..64]) };
    let p2 = SyscallPoint256 { x: read_coord(&input[64..96]), y: read_coord(&input[96..128]) };

    let mut params = SyscallBabyJubJubAddParams { p1: &mut p1, p2: &p2 };
    syscall_babyjubjub_add(&mut params);

    let mut out = [0u8; 64];
    for (i, limb) in p1.x.iter().enumerate() {
        out[i * 8..i * 8 + 8].copy_from_slice(&limb.to_le_bytes());
    }
    for (i, limb) in p1.y.iter().enumerate() {
        out[32 + i * 8..32 + i * 8 + 8].copy_from_slice(&limb.to_le_bytes());
    }
    ziskos::io::commit_slice(&out);
}
