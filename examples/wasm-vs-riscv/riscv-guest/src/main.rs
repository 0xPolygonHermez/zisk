//! RISC-V guest: the same program against `ziskos`. Commits the exact bytes the wasm guest
//! prints, so the two public outputs can be compared.

#![no_main]
ziskos::entrypoint!(main);

fn keccak_f(state: &mut [u64; 25]) {
    unsafe { ziskos::syscalls::syscall_keccak_f(state) }
}

fn main() {
    let input = ziskos::io::read_slice();
    let id = u64::from_le_bytes(input[..8].try_into().unwrap());
    let n = u64::from_le_bytes(input[8..16].try_into().unwrap());
    let sum = workloads::run(id, n, keccak_f);
    let text = format!("{} {n} {sum:016x}\n", workloads::NAMES[id as usize]);
    ziskos::io::commit_slice(text.as_bytes());
    print!("{text}");
}
