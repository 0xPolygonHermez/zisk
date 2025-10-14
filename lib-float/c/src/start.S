.section .text.init
.global _start
_start:
    # Set up global pointer
    .option push
    .option norelax
    la gp, _global_pointer
    .option pop
    
    # Set up stack pointer 
    la sp, _init_stack_top
    
    # Call main function
    call _zisk_main
    
    # Handle return from main
    # csrr t0, 0xf12
    # li   t1, 0xFFFEEEE
    # beq  t0, t1, 1f
    
    # QEmu exit for regular simulation
    # li t0, 0x100000
    # li t1, 0x5555  
    # sw t1, 0(t0)
    # j 2f
    
1: # ZisK specific exit
    li a7, 93      # exit system call number
    ecall
    
2: # End loop
    j 2b
