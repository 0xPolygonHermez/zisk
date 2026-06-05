#![no_main]
ziskos::entrypoint!(main);

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod riscv_c;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod riscv_fd;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod riscv_ima;

mod fcalls;
mod syscalls;

fn main() {
    // Basic instructions
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        riscv_c::diagnostic_riscv_c();
        riscv_fd::diagnostic_riscv_fd();
        riscv_ima::diagnostic_riscv_ima();
        //riscv_ima::diagnostic_riscv_ima_combinations(); // TODO
    }

    // Free-input calls (hinting)
    fcalls::diagnostic_bigint();
    fcalls::diagnostic_bls12_381();
    fcalls::diagnostic_bn254();
    fcalls::diagnostic_fcall_limits();
    fcalls::diagnostic_msb();
    fcalls::diagnostic_secp256k1();
    fcalls::diagnostic_secp256r1();
    fcalls::diagnostic_uint256();

    // System calls
    syscalls::diagnostic_arith256();
    syscalls::diagnostic_arith384();
    syscalls::diagnostic_blake2();
    syscalls::diagnostic_bls12_381();
    syscalls::diagnostic_bn254();
    syscalls::diagnostic_keccakf();
    syscalls::diagnostic_poseidon2();
    syscalls::diagnostic_secp256k1();
    syscalls::diagnostic_secp256r1();
    syscalls::diagnostic_sha256f();

    println!("Successfully completed all diagnostics!");
}
