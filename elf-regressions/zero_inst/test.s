# Zero Instruction Skipping Bug Test
# 
# This test replicates the cpuinit bug where skipping a zero instruction
# causes a CSRRW instruction (0xc0001073) to appear at the wrong address (this is a trap)

.section .text.init
.global _start

_start:
    # Instruction at 0x80000000 (index 0)
    li a0, 1            
    
    # Instruction at 0x80000004 (index 1)
    li a1, 2            
    
    # NOP at 0x80000008 (index 2) - proper RISC-V nop instruction
    nop    
    
    # JAL at 0x8000000c (should be index 3, becomes index 2 if zero skipped)
    jal ra, target
    
    # CSRRW at 0x80000010 (should be index 4, becomes index 3 if zero skipped)
    .word 0xc0001073
    
    # This should also not execute, since `target` exits
    li a2, 99           

target:
    # Target at 0x80000018 (or wrong address if zero skipped)
    # If the zero is not skipped, we jump here correctly
    # If the zero is skipped, the jump calculation is currently wrong
    
    # Exit successfully 
    li a7, 93           # exit syscall
    li a0, 0            # exit code  -- not needed though
    ecall
    
    # Infinite loop to prevent falling off the end
loop:
    j loop