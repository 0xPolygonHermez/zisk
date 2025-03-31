use core::arch::asm;

use crate::SYSCALL_ARITH256_MOD_ID;

/// Raises `x` to (2^power_log) modulo `module` using repeated squaring
/// Performs all operations in RISC-V assembly for maximum performance
pub fn exp_power_of_two(x: &[u64; 4], module: &[u64; 4], power_log: usize) -> [u64; 4] {
    let mut result = *x;

    unsafe {
        asm!(
            // ===== STACK SETUP =====
            // Reserve stack space (96 bytes total: 80 for struct + 16 padding)
            "addi sp, sp, -96",       // Adjust stack pointer
            "andi sp, sp, -16",       // Align to 16-byte boundary (ABI requirement)

            // ===== INITIALIZE ZERO ARRAY =====
            // We'll use this for the c parameter (must be zeros)
            "sd zero, 64(sp)",        // Initialize zero[0] = 0
            "sd zero, 72(sp)",        // Initialize zero[1] = 0
            "sd zero, 80(sp)",        // Initialize zero[2] = 0
            "sd zero, 88(sp)",        // Initialize zero[3] = 0

            // ===== SETUP POINTER ARRAY =====
            // Structure layout on stack:
            // 0: a (input) = x
            // 8: b (input) = x
            // 16: c (must be zero) = &zero
            // 24: module = provided module
            // 32: d (output) = x (in-place modification)
            "sd {x_ptr}, 0(sp)",      // Store a = x pointer
            "sd {x_ptr}, 8(sp)",      // Store b = x pointer
            "addi t0, sp, 64",        // Get address of zero array
            "sd t0, 16(sp)",          // Store c = &zero
            "sd {mod_ptr}, 24(sp)",   // Store module pointer
            "sd {res_ptr}, 32(sp)",   // Store d = result pointer

            // ===== EXPONENTIATION LOOP =====
            // t1 will be our loop counter (power_log)
            "mv t1, {power_log}",    // Initialize counter
            "1:",                    // Start of loop
            "beqz t1, 2f",           // If counter == 0, exit loop

            // ===== PERFORM MODULAR SQUARING =====
            // Call emulator's internal function via CSR 0x802
            // The CSR reads our prepared structure from the stack
            "csrrs zero, {arith256_mod_id}, sp",  // syscall to arith256_mod

            // Update pointers for next iteration:
            // After first squaring, we want to square the result
            "sd {res_ptr}, 0(sp)",    // Update a to point to result
            "sd {res_ptr}, 8(sp)",    // Update b to point to result

            // ===== LOOP CONTROL =====
            "addi t1, t1, -1",       // Decrement counter
            "j 1b",                  // Jump back to start of loop

            // ===== CLEANUP =====
            "2:",
            // Restore stack pointer
            "addi sp, sp, 96",

            // ===== INPUT PARAMETERS =====
            x_ptr = in(reg) x.as_ptr(),
            mod_ptr = in(reg) module.as_ptr(),
            res_ptr = in(reg) result.as_mut_ptr(),
            power_log = in(reg) power_log,
            arith256_mod_id = const SYSCALL_ARITH256_MOD_ID,
            // ===== TEMPORARY REGISTERS =====
            out("t0") _,
            out("t1") _,

            // ===== ASSEMBLY CONSTRAINTS =====
            options(nostack)  // We manage stack manually
        );
    }

    result
}

/// Raises `x` to (2^power_log) modulo `module` using repeated squaring
/// Performs all operations in RISC-V assembly for maximum performance
pub fn exp_power_of_two_self(x: &mut [u64; 4], module: &[u64; 4], power_log: usize) {
    unsafe {
        asm!(
            // ===== STACK SETUP =====
            // Reserve stack space (96 bytes total: 80 for struct + 16 padding)
            "addi sp, sp, -96",       // Adjust stack pointer
            "andi sp, sp, -16",       // Align to 16-byte boundary (ABI requirement)

            // ===== INITIALIZE ZERO ARRAY =====
            // We'll use this for the c parameter (must be zeros)
            "sd zero, 64(sp)",        // Initialize zero[0] = 0
            "sd zero, 72(sp)",        // Initialize zero[1] = 0
            "sd zero, 80(sp)",        // Initialize zero[2] = 0
            "sd zero, 88(sp)",        // Initialize zero[3] = 0

            // ===== SETUP POINTER ARRAY =====
            // Structure layout on stack:
            // 0: a (input) = x
            // 8: b (input) = x
            // 16: c (must be zero) = &zero
            // 24: module = provided module
            // 32: d (output) = x (in-place modification)
            "sd {x_ptr}, 0(sp)",      // Store a = x pointer
            "sd {x_ptr}, 8(sp)",      // Store b = x pointer
            "addi t0, sp, 64",        // Get address of zero array
            "sd t0, 16(sp)",          // Store c = &zero
            "sd {mod_ptr}, 24(sp)",   // Store module pointer
            "sd {x_ptr}, 32(sp)",     // Store d = x (output will overwrite input)

            // ===== EXPONENTIATION LOOP =====
            // t1 will be our loop counter (power_log)
            "mv t1, {power_log}",    // Initialize counter
            "1:",                    // Start of loop
            "beqz t1, 2f",           // If counter == 0, exit loop

            // ===== PERFORM MODULAR SQUARING =====
            // Call emulator's internal function via CSR 0x802
            // The CSR reads our prepared structure from the stack
            "csrrs zero, {arith256_mod_id}, sp",  // syscall to arith256_mod

            // ===== LOOP CONTROL =====
            "addi t1, t1, -1",       // Decrement counter
            "j 1b",                  // Jump back to start of loop

            // ===== CLEANUP =====
            "2:",
            // Restore stack pointer
            "addi sp, sp, 96",

            // ===== INPUT PARAMETERS =====
            x_ptr = in(reg) x.as_mut_ptr(),
            mod_ptr = in(reg) module.as_ptr(),
            power_log = in(reg) power_log,
            arith256_mod_id = const SYSCALL_ARITH256_MOD_ID,
            // ===== TEMPORARY REGISTERS =====
            out("t0") _,
            out("t1") _,

            // ===== ASSEMBLY CONSTRAINTS =====
            options(nostack)  // We manage stack manually
        );
    }
}
