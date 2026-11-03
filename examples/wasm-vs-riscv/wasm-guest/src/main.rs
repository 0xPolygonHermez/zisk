//! WebAssembly guest: a plain `wasm32-wasip1` program. Reads the workload id and size (two
//! little-endian `u64`s) from stdin and prints the checksum to stdout, the public output.

use std::io::Read;

#[link(wasm_import_module = "env")]
extern "C" {
    /// The Keccak-f precompile (see transpilers/wasm/README.md, "The raw precompiles").
    #[link_name = "keccak_f"]
    fn keccak_f_import(state: *mut u64);
}

fn keccak_f(state: &mut [u64; 25]) {
    unsafe { keccak_f_import(state.as_mut_ptr()) }
}

fn main() {
    let mut buf = [0u8; 16];
    std::io::stdin().read_exact(&mut buf).expect("input: workload id and size");
    let id = u64::from_le_bytes(buf[..8].try_into().unwrap());
    let n = u64::from_le_bytes(buf[8..].try_into().unwrap());
    let sum = workloads::run(id, n, keccak_f);
    println!("{} {n} {sum:016x}", workloads::NAMES[id as usize]);
}
