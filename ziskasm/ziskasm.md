# ZisK assembly syntax specificiation


## Introduction

This document specifies the syntax of the ZisK assembly language, that allows to write ZisK code
directly, without having to cross different compilation and transpilation processes, e.g. rust -> RISC-V -> ZisK.
This removes the limitations that RISC-V has vs. ZisK, and allows using the ZisK instructions
functionality at its maximum capacity, decreasing the number of execution steps and the proof
generation cost.

## File format

A ZisK assembly file must:
- Be a plain text file
- Have the .zisk extension
- Have one text line per definition, per label, and per instruction
- Empty lines are allowed, to improve readability
- Definitions will be idented at the beginning of the text line
- Labels will be idented at the beginning of the text line and will be sufixed with a colon character
- Instructions will be prefixed with a tab character, to improve readability of the code
- Comments will be prefixed with a semi-colon character.  Any characters after a semi-colon character will be ignored in the parsing process, except to add the comment as debug data

This is an example:

```
; <general comment>

<definition> ; <definition comment>

<label>: ; <label comment>
    <instruction> ; <instruction comment>
    <instruction> ; <instruction comment>
```

## Number format

Numbers are specified as integer numbers, either in decimal, or in hexagesimal with the `0x` prefix.  When specified in hexagesimal, both lower-case and upper-case are allowed to be used in the alphanumeric digits.

## Instruction format

The instruction must follow a format with some fields that are mandatory, and some fields that are optional.  This is the generic instruction format:

```
operation(a_source, b_source) -> c_storage, j(jump1, jump2), setpc(jump), sp, end
```

The instruction must contain all the information required to populate an instance of the ZiskInst element specified in the file zisk_inst.rs.

An instruction addres is assigned inside the ROM address range (check the file mem.rs) starting at address ROM_ADDR.

Two consecutive instructions are separated by 4 address bytes, i.e. next_instruction_pc = current_pc + 4.

### Register format

In some cases a general-purpose register is used to load or to store data.

A register will be noted as `rN`, where N is the register number in decimal format.

The maximum value of N is 34.

The register `r0` is always read as zero, regardless of any previous value written to it.  This is the same behavior as RISC-V.

The registers `r1` to `r31` are the same as the RISC-V registers, and they are not stored in memory in order to increase performance.

The registers `r32` to `r34` are stored in memory.

### Memory format

In some cases memory must be used to load or to store data.  There are several ways to specify how this memory operation is performed.

The format `[N]`, where N is a literal number either in decimal format or in hexagesimal format, refers to the memory value addressed by N.

The formats `[a + N]` and `[a - N]`, where N is a literal number in decimal format, refers to the memory value addressed by the value of the `a` register plus/minus the value of N.  The value of N, with sign, is stored in the ZiskInst instance field `b_offset_imm0`.

### Operation field

This field is mandatory.

The `operation` field is a string that describes the ZisK operation.  We will use the ZiskOps name field, e.g. "copyb", "add", "sll_w", etc.  Check the file zisk_ops.rs for the complete list.

The result of the operation is stored in registers `c` and `flag`.

The register `c` persists to be used as an `a_source` in the next instruction.

The register `flag` can be used to jump to the next instruction based on the field `j(jump1, jump2)`, if used.

### Source fields

These fields are mandatory.  You can set them to 0 if they are not used in the corresponding operation.

The `(a_source, b_source)` fields describe how the registers a and b are loaded

The possible formats of `a_source` can be:
- `c`, meaning that register `a` will be loaded with the current value of register `c`, i.e. with the result of the previously executed instruction.  The ZiskInst instance field `a_src` is set to SRC_C.
- `rN`, meaning that register `a` will be loaded with the current value of register `rN`.  The ZiskInst instance field `a_src` is set to SRC_REG, and the field `a_offset_imm0` is set to N.
- `[N]`, meaning that register `a` will be loaded with the current value of the memory at address N, plus the SP register if specified.  The ZiskInst instance field `a_src` is set to SRC_MEM, and the field `a_offset_imm0` is set to N.
- `N`, meaning that register `a` will be loaded with the value N, which is a u64.  The ZiskInst instance field `a_src` is set to SRC_IMM, the field `a_offset_imm0` is set to the lower 32 bits of N, and the field `a_use_sp_imm1` is set to the higher 32 bits of N.
- `step`, meaning that register `a` will be loaded with the current step number, i.e. the number of instructions executed up to this point. step=0 means the first instruction to execute.  The ZiskInst instance field `a_src` is set to SRC_STEP.

The possible formats of `b_source` can be:
- `c`, meaning that register `b` will be loaded with the current value of register `c`, i.e. with the result of the previously executed instruction.  The ZiskInst instance field `b_src` is set to SRC_C.
- `rN`, meaning that register `b` will be loaded with the current value of register `rN`.  The ZiskInst instance field `b_src` is set to SRC_REG, and the field `b_offset_imm0` is set to N.
- `[N]`, meaning that register `b` will be loaded with the current value of the memory at address N, plus the SP register if specified.  The ZiskInst instance field `b_src` is set to SRC_MEM, and the field `b_offset_imm0` is set to N.
- `N`, meaning that register `b` will be loaded with the value N, which is a u64.  The ZiskInst instance field `b_src` is set to SRC_IMM, the field `b_offset_imm0` is set to the lower 32 bits of N, and the field `b_use_sp_imm1` is set to the higher 32 bits of N.
- `W[a+N]`, meaning that register `b` will be loaded with the first W bytes of current value of the memory at address equals the value of `a` register plus the value of N, which can be a negative offset.  The ZiskInst instance field `b_src` is set to SRC_IND, and the ZiskInst instance field `b_offset_imm0` is set to the address offset, i.e. to N including sign, and the field `ind_width` is set to W, which can be 1, 2, 4 or 8.

### Storage field

This field is optional.

The field ` -> c_storage` specifies how the `c` register must be stored, either in a register, or in memory.

When `c_storage` is a register (e.g. `r10`) then the value of the register c is stored in the specified register, after executing the operation.  The register number (e.g. 10 in the last example) is stored in the field `store_offset` of the ZiskInst instance.

When `c_storage` is a memory location (e.g. `[0x9004000]`) the value of register c is stored in the memory at the specified address (plus the sp register if specified).

### Jump field

This field is optional.

The field `, j(jump1, jump2)` specifies how the program counter (`pc`) must be updated after the execution of the operation in order to jump to the next instruction.

The fields `jump1` and `jump2` can be either a signed integer number (i64), in which case they refer to the offset of the target instruction to jump to vs. the current pc, or they can be an instruction label.

If after the operation execution the flag register equals 1, then the next instruction is the one referred by `jump1`.  If the flag register equals 0, then the next instruction is the one referred by `jump2`.

When `jump2` refers to the next instruction (i.e. current_pc + 4) then it can be omitted and this field can be simplified to `, j(jump1)`;

When the next instruction to execute is always current_pc + 4, this field can be omitted.

### Set PC field

This field is optional.

The field `, setpc(offset)` specifies what the next instruction pc will be, i.e. what the next instruction to execute is.  The ZiskInst instance field `set_pc` is set to true.

The field `offset` is a signed integer (i64), and is stored in the ZiskInst instance field `jump_offset1`.  The next instruction pc will be the `c` register value plus the `offset` field value.

In other words:

```
next_instruction_pc = c + offset
```

### SP field

This field is optional.

The field `, sp` specifies that register `sp` must be taken into account when calculating some address operations.

This field sets the field `use_sp` of the ZiskInst instance to true.

### End field

This field is optional.

The field `end` specifies that this is the last instruction of the program to be executed.

This field sets the field `end` of the ZiskInst instance to true.

<!--

Operation: enum
Source a: enum + imm
Source b: enum + imm
Store c: enum + imm
Set pc (bool)
End: bool
Jump 1: i64 (address offset)
Jump 2: i64 (address offset)

Registers:

Logical instruction size (bits between 2 consecutive addresses)
Constant values (definitions)
Constant data
Variable data
call / ret :
what register contains return address (same as RISC-V)
what registers to save (ideally, only those used by callee)
How to identify main label (e.g. “main:”)
Tabs
How to split code between different files:
imports or includes of other files
external functions and data
”Makefile” with a list of files to compile…
How to integrate BIOS code, e.g. how to do syscall
How to call read_input / write_output / simply access the input/output memory address
How to call precompiles
Hints / pragmas

-->