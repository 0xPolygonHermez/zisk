# Tests that JALR works when code doesn't start at 0x80000000
#
# This is mainly testing `jumpt_to_dynamic_pc`; it currently assumes that
# user code starts from 0x800000000.

# Header section - emulating ELF/program headers in first page
.section .header, "a"
.align 12

# Simulated ELF header (first 64 bytes)
.byte 0x7f, 'E', 'L', 'F'     # ELF magic
.byte 2, 1, 1, 0               # 64-bit, little-endian, version 1
.zero 8                        # padding
.hword 2                       # e_type: EXEC
.hword 0xf3                    # e_machine: RISC-V
.word 1                        # e_version
.quad 0x80001000              # e_entry: entry point
.quad 0x40                    # e_phoff: program header offset
.quad 0                       # e_shoff: section header offset
.word 0                       # e_flags
.hword 64                     # e_ehsize: ELF header size
.hword 56                     # e_phentsize: program header entry size
.hword 1                      # e_phnum: number of program headers
.hword 0                      # e_shentsize
.hword 0                      # e_shnum
.hword 0                      # e_shstrndx

# Simulated Program Header (at offset 0x40)
.word 1                       # p_type: PT_LOAD
.word 5                       # p_flags: PF_R | PF_X
.quad 0x1000                  # p_offset
.quad 0x80000000             # p_vaddr
.quad 0x80000000             # p_paddr
.quad 0x2000                  # p_filesz
.quad 0x2000                  # p_memsz
.quad 0x1000                  # p_align

# Fill rest of the page with zeros
.zero 0x1000 - (. - .header)

.section .text.init
.global _start

_start:    
    # Simple JALR test
    la t0, target       # Load address of target
    jalr ra, t0, 0      # Jump and link to target
    
    # If we get here, JALR worked
    li a0, 42           # Success value
    
    # Exit
    li a7, 93
    ecall

target:
    li a1, 100
    ret                 # This is also a JALR