#![no_main]
ziskos::entrypoint!(main);

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod riscv_c;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod riscv_fd;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod riscv_ima;

mod accelerators;
mod fcalls;
mod syscalls;

fn main() {
    // Base RISC-V extensions
    #[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
    {
        riscv_c::diagnostic_riscv_c();
        riscv_fd::diagnostic_riscv_fd();
        riscv_ima::diagnostic_riscv_ima();
        //riscv_ima::diagnostic_riscv_ima_combinations(); // TODO
    }

    // System calls
    syscalls::diagnostic_syscalls();

    // Free-input calls (hinting)
    fcalls::diagnostic_fcalls();

    // Accelerators
    accelerators::diagnostic_accelerators();

    println!("Successfully completed all diagnostics!");
}
