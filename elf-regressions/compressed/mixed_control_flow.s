# Mixed compressed control flow operations test
# Tests combinations of branches, jumps, and function calls

.section .text.init
.global _start

_start:
    # Test nested conditional branches
    li x8, 10
    li x9, 0
    li x10, 5
    
    c.bnez x8, branch1   # Should branch (x8 = 10)
    li x1, 0xbad
    c.j error
    
branch1:
    c.beqz x9, branch2   # Should branch (x9 = 0)
    li x2, 0xbad
    c.j error
    
branch2:
    c.bnez x10, branch3  # Should branch (x10 = 5)
    li x3, 0xbad
    c.j error
    
branch3:
    li x1, 0x100
    li x2, 0x200
    li x3, 0x300
    
    # Test function call with conditional returns
    li x8, 1             # Test condition
    la x11, conditional_func
    c.jalr x11
    
    li x4, 0x400         # Mark return point
    
    # Test loop with compressed branches
    li x12, 5            # Loop counter
    li x13, 0            # Accumulator
    
loop1:
    c.beqz x12, loop1_end
    c.add x13, x12       # Add counter to accumulator
    c.addi x12, -1       # Decrement counter
    c.j loop1
    
loop1_end:
    # x13 should be 15 (5+4+3+2+1)
    
    # Test nested loops
    li x14, 3            # Outer counter
    li x15, 0            # Result accumulator
    
outer_loop:
    c.beqz x14, outer_end
    
    # Inner loop (use compressed registers)
    li x8, 2             # Inner counter (use compressed register)
    
inner_loop:
    c.beqz x8, inner_end # c.beqz only works on x8-x15
    c.add x15, x14       # Add outer counter to result
    c.addi x8, -1        # Decrement inner counter (c.addi only on x8-x15)
    c.j inner_loop
    
inner_end:
    c.addi x14, -1       # Decrement outer counter
    c.j outer_loop
    
outer_end:
    # x15 should be 3*2 + 2*2 + 1*2 = 12
    
    # Test switch-like construct using jumps
    li x9, 2             # Switch value (use compressed register)
    
    # Switch implementation
    c.beqz x9, case0
    li t0, 1
    beq x9, t0, case1
    li t0, 2
    beq x9, t0, case2
    c.j default_case
    
case0:
    li x18, 0xc0
    c.j switch_end
    
case1:
    li x18, 0xc1
    c.j switch_end
    
case2:
    li x18, 0xc2
    c.j switch_end
    
default_case:
    li x18, 0xcf
    
switch_end:
    # x18 should be 0xc2
    
    # Test recursive-like pattern with return address management
    li x19, 3            # Recursion depth
    la x20, recursive_func
    c.jalr x20
    
    li x5, 0x500         # Mark after recursion
    
    # Test exception-like control flow
    la x21, protected_code
    c.jalr x21           # "Call" protected code
    
    # Exception handler simulation
    li x6, 0x600
    c.j continue_after_exception
    
exception_handler:
    li x6, 0x6ee         # Mark exception handled
    c.jr x1              # "Return" from exception
    
protected_code:
    # Simulate some code that might "throw"
    li x14, 1            # Use compressed register
    c.bnez x14, simulate_exception
    
    # Normal path
    li x23, 0x700
    c.jr x1
    
simulate_exception:
    # Jump to exception handler
    la x24, exception_handler
    c.jr x24
    
continue_after_exception:
    # Test computed jump simulation
    li x13, 1            # Jump table index (use compressed register)
    
    # Simple jump table using branches
    c.beqz x13, jump_target0
    li t0, 1
    beq x13, t0, jump_target1
    li t0, 2
    beq x13, t0, jump_target2
    c.j jump_target_default
    
jump_target0:
    li x26, 0x100        # Valid hex constant (was 0xj0)
    c.j jump_table_end
    
jump_target1:
    li x26, 0x101        # Valid hex constant (was 0xj1)
    c.j jump_table_end
    
jump_target2:
    li x26, 0x102        # Valid hex constant (was 0xj2)
    c.j jump_table_end
    
jump_target_default:
    li x26, 0x10f        # Valid hex constant (was 0xjf)
    
jump_table_end:
    # x26 should be 0x101
    
    # Test early return pattern
    li x27, 0
    la x28, early_return_func
    c.jalr x28
    
    li x7, 0x700
    
    # Verification
    c.j verify_results

conditional_func:
    # Function with conditional execution
    c.bnez x8, cond_true
    li x29, 0xfa15e      # Valid hex constant (was 0xfa1se)
    c.jr x1
    
cond_true:
    li x29, 0x701e       # Valid hex constant (was 0x7r0e)
    c.jr x1

recursive_func:
    # Simulate recursive function (use compressed register)
    c.beqz x10, recursive_base   # x19 -> x10 (compressed register)
    
    # Recursive case
    c.addi x10, -1       # Decrement depth (use compressed register)
    c.add x11, x10       # Add current depth to result (compressed registers)
    c.jr x1              # Return (simplified recursion)
    
recursive_base:
    # Base case
    li x11, 0x800        # Base result
    c.jr x1

early_return_func:
    # Function with early return
    c.bnez x12, early_ret    # Use compressed register x12
    
    # Normal execution
    li x31, 0x900
    c.jr x1
    
early_ret:
    # Early return path
    li x31, 0x901
    c.jr x1

verify_results:
    # Verify test results
    li t0, 0x100
    bne x1, t0, error
    
    li t0, 0x200
    bne x2, t0, error
    
    li t0, 0x300
    bne x3, t0, error
    
    li t0, 0x400
    bne x4, t0, error
    
    li t0, 0x500
    bne x5, t0, error
    
    li t0, 0x6ee
    bne x6, t0, error
    
    li t0, 0x700
    bne x7, t0, error
    
    # Check computed values
    li t0, 15
    bne x13, t0, error   # Loop sum
    
    li t0, 12
    bne x15, t0, error   # Nested loop result
    
    li t0, 0xc2
    bne x18, t0, error   # Switch result
    
    li t0, 0x101
    bne x26, t0, error   # Jump table result
    
    # Success
    li a0, 0
    li a7, 93
    ecall

error:
    li a0, 1
    li a7, 93
    ecall
    
1:  j 1b