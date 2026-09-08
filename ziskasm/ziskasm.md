# ZisK assembly syntax specification

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
- Definitions will be written at the beginning of the text line (not indented)
- Labels will be written at the beginning of the text line (not indented) and will be suffixed with a colon character
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

Numbers are specified as integer numbers, either in decimal, or in hexadecimal with the `0x` prefix.  When specified in hexadecimal, both lower-case and upper-case are allowed to be used in the alphanumeric digits.

Wherever a number is expected as an immediate source (`N`), a memory address (`[N]`), or a jump target, an identifier naming a **symbol** — a label or a data declaration (see [Data declarations](#data-declarations)) — may be used instead.  The assembler resolves the symbol to its address.  For example, `copyb(0, counter) -> r5` loads the *address* of the data symbol `counter`, and `[counter]` reads its *value*.

## Register format

In some cases a general-purpose register is used to load or to store data.

A register will be noted as `rN`, where N is the register number in decimal format.

The maximum value of N is 31, i.e. the general-purpose registers are `r0` to `r31` (the RISC-V `x0` to `x31`).

The register `r0` is always read as zero, regardless of any previous value written to it, and a store to `r0` is discarded.  This is the same behavior as RISC-V.  Note that the assembler encodes `r0` as an immediate value of 0 (`SRC_IMM`), not as `SRC_REG`.

The registers `r1` to `r31` are the same as the RISC-V registers, and they are kept in the main execution trace (not in memory) in order to increase performance.

## Virtual registers

ZisK supports 32 extra virtual registers, meaning that they are transpiled as memory reads and writes into the registers area.  They are slower than regular, RISC-V-based registers, but convenient when you need temporary storage.  In oher words, `rN` with 32 <= `N` <= 63 will be interpreted as memory accesses to a system memory region dedicated to this purpose.

## Memory format

In some cases memory must be used to load or to store data.  There are several ways to specify how this memory operation is performed.

The format `[N]`, where N is a literal number either in decimal format or in hexadecimal format, refers to the memory value addressed by N.

The formats `[a + N]` and `[a - N]`, where N is a literal number in decimal format or hexadecimal format, refers to the memory value addressed by the value of the `a` register plus/minus the value of N.  The value of N is stored in the corresponding ZiskInst instance field with sign.

The format `W[a + N]` and `W[a - N]`, where N is a literal number in decimal format or hexadecimal, refers to the memory value addressed by the value of the `a` register plus/minus the value of N.  The value of N is stored in the corresponding ZiskInst instance field with sign.  W is a literal number that refers to how many bytes must be copied when accessing to memory, and can only take the values 1, 2, 4 and 8.

## Definition format

A definition is used to assign a text value to a text identifier.  This is the generic definition format:

```
define TEXT_IDENTIFIER text_value
```

After the line of the definition, if the text idenfifier is found, it is replaced by its corresponding text value.  There is not type check, just a text replacement.

Example:

```
define RAM_MEM 0x9000000
```

A plain `define` is **file-local**: it is only visible in the file that declares
it. In a multi-file assembly (a library, or a `-z <dir>` run) a definition can be
made visible to **every** file by prefixing it with `pub`:

```
pub define FREE_INPUT 0x40000000
```

The assembler gathers all `pub define`s across the sources in a pre-pass and seeds
every file's parse with them, so a constant used by several files can be declared
once (typically in a shared/common file) instead of repeated in each. A `pub
define` still behaves like a normal `define` within its own file. Publicly
defining the same name twice with **different** values is an error. (`pub` has no
effect in a single-file program — there are no sibling files to export to.)

## Conditional compilation

Parts of a program can be included or excluded at assembly time with directives written at the beginning of the line (not indented), like definitions:

```
ifdef NAME
    ; ... assembled only if NAME is defined ...
else
    ; ... assembled only if NAME is not defined ...
endif

ifndef NAME
    ; ... assembled only if NAME is not defined ...
endif
```

`else` is optional, and `ifdef`/`ifndef` blocks may be nested.  A symbol `NAME` is considered *defined* if it was introduced with `define NAME ...` earlier in the program, or if it was predefined externally by the assembler (see below).

The assembler predefines a symbol to select the build target:

- **`ASM`** is defined when assembling for the x86 assembly generator (the `zisk2zisk` tool); it is not defined for the Rust emulator (`ziskemu -z`).  Some ZisK ops exist only in the Rust emulator and are not emitted by the x86 generator (the Zba/Zbc/Zbkx/Zicond ops: `sh*add`, `add_u_w`, `sll_u_w`, `clmul*`, `xperm*`, `czero_*`).  A program that uses them should guard them with `ifndef ASM` so the x86 build excludes them.  The diagnostic program does exactly this.

## Label format

A label is used to assign a text identifier to the program address of the next instruction.  This is the generic label format:

```
TEXT_IDENTIFIER:
```

The `TEXT_IDENTIFIER` field must start with a letter, and must be unique in the context of the ZisK assembly program.

The label can be used in order to jump to that instruction from another instruction.

## Data declarations

A data declaration reserves initialized storage and binds a name (a symbol) to its address.  Like labels, data symbols are resolved globally by the assembler, so a declaration may appear anywhere in the program (before or after its uses) and in any file.  A data declaration is written at the beginning of the line (not indented), like a definition or a label.

The generic format is:

```
[const] TYPE NAME[SIZE] = value0, value1, ...
```

where:

- `const` (optional): if present, the data is **read-only** and is stored in ROM, right after the program code.  If absent, the data is **read-write** and is stored in RAM (starting at `GENERAL_RAM_ADDR`; see mem.rs).
- `TYPE`: one of `u8`, `u16`, `u32` or `u64`.  Every element occupies **one 8-byte slot** regardless of the type; the type only range-checks the initial values (and documents intent).  A value that does not fit in `TYPE` is an error.
- `NAME`: the symbol bound to the address of the data.  It must be a valid identifier and unique across all labels and data names.
- `[SIZE]` (optional): the number of elements (an array).  If omitted, the declaration is a scalar (one element), unless a value list longer than one is given, in which case `SIZE` is inferred from the list.
- `= value0, value1, ...` (optional): the initial values, one per element.  If fewer values than `SIZE` are given, the remaining elements are zero.  If no values are given, all elements are zero.

All elements are 8-byte aligned (each occupies an 8-byte slot).  Initial values must be literal numbers.

Examples:

```
const u64 MAGIC = 0xdeadbeef        ; scalar constant in ROM
const u64 TABLE[4] = 10, 20, 30, 40 ; array constant in ROM
const u32 PRIMES = 2, 3, 5, 7, 11   ; array of 5 (size inferred) in ROM
u64 counter = 0                     ; scalar variable in RAM
u64 buffer[64]                      ; zero-initialized array in RAM
```

### Using data symbols

A data symbol resolves to the **address** of its storage (like a label).  So:

- `NAME` used as an immediate or address operand is the **address** of the data (a pointer to element 0).
- `[NAME]` reads or writes the **value** of element 0 (an 8-byte memory access at `NAME`).
- To access array element `i`, load the base address into a register (`copyb(0, NAME) -> rP`) and use an indirect operand (`W[a + N]`) with that register as the base (the same pattern the doubler example uses for the input and output arrays).

Example (summing a constant array into a RAM accumulator):

```
const u64 TABLE[4] = 10, 20, 30, 40
u64 acc = 0
    copyb(0, TABLE) -> r10       ; r10 = address of TABLE (a pointer)
    copyb(0, [acc]) -> r7        ; r7 = value of acc (0)
    copyb(r10, 8[a + 0]) -> r8   ; r8 = TABLE[0]
    add(r7, r8) -> r7            ; acc += TABLE[0]
```

## Instruction format

The instruction must follow a format with some fields that are mandatory, and some fields that are optional.  This is the generic instruction format:

```
operation(a_source, b_source) -> c_storage, j(jump1, jump2), setpc(jump), sp, end
```

The instruction must contain all the information required to populate an instance of the ZiskInst element specified in the file zisk_inst.rs.

An instruction addres is assigned inside the ROM address range (check the file mem.rs) starting at address ROM_ADDR.

Two consecutive instructions are separated by 4 address bytes, i.e. next_instruction_pc = current_pc + 4.

## Operation field

This field is mandatory.

The `operation` field is a string that describes the ZisK operation.  We will use the ZiskOps name field, e.g. "copyb", "add", "sll_w", etc.  Check the file zisk_ops.rs for the complete list.

The result of the operation is stored in registers `c` and `flag`.

The register `c` persists to be used as an `a_source` in the next instruction.

The register `flag` can be used to jump to the next instruction based on the field `j(jump1, jump2)`, if used.

## Source fields

These fields are mandatory.  You can set them to 0 if they are not used in the corresponding operation.

The `(a_source, b_source)` fields describe how the registers a and b are loaded

The possible formats of `a_source` are:
- `c`, meaning that register `a` will be loaded with the current value of register `c`, i.e. with the result of the previously executed instruction.  The ZiskInst instance field `a_src` is set to SRC_C.
- `rN`, meaning that register `a` will be loaded with the current value of register `rN`.  The ZiskInst instance field `a_src` is set to SRC_REG, and the field `a_offset_imm0` is set to N.
- `[N]`, meaning that register `a` will be loaded with the current value of the memory at address N, plus the SP register if specified.  The ZiskInst instance field `a_src` is set to SRC_MEM, and the field `a_offset_imm0` is set to N.
- `N`, meaning that register `a` will be loaded with the value N, which is a u64.  The ZiskInst instance field `a_src` is set to SRC_IMM, the field `a_offset_imm0` is set to the lower 32 bits of N, and the field `a_use_sp_imm1` is set to the higher 32 bits of N.
- `step`, meaning that register `a` will be loaded with the current step number, i.e. the number of instructions executed up to this point. step=0 means the first instruction to execute.  The ZiskInst instance field `a_src` is set to SRC_STEP.

The possible formats of `b_source` are:
- `c`, meaning that register `b` will be loaded with the current value of register `c`, i.e. with the result of the previously executed instruction.  The ZiskInst instance field `b_src` is set to SRC_C.
- `rN`, meaning that register `b` will be loaded with the current value of register `rN`.  The ZiskInst instance field `b_src` is set to SRC_REG, and the field `b_offset_imm0` is set to N.
- `[N]`, meaning that register `b` will be loaded with the current value of the memory at address N, plus the SP register if specified.  The ZiskInst instance field `b_src` is set to SRC_MEM, and the field `b_offset_imm0` is set to N.
- `N`, meaning that register `b` will be loaded with the value N, which is a u64.  The ZiskInst instance field `b_src` is set to SRC_IMM, the field `b_offset_imm0` is set to the lower 32 bits of N, and the field `b_use_sp_imm1` is set to the higher 32 bits of N.
- `W[a+N]`, meaning that register `b` will be loaded with the first W bytes of current value of the memory at address equals the value of `a` register plus the value of N, which can be a negative offset.  The ZiskInst instance field `b_src` is set to SRC_IND, and the ZiskInst instance field `b_offset_imm0` is set to the address offset, i.e. to N including sign, and the field `ind_width` is set to W, which can be 1, 2, 4 or 8.

Notes on register operands:
- Only `a_source` can use `step`; `b_source` cannot (there is no SRC_STEP for register `b`).
- The register `r0` is encoded as the immediate value 0 (`SRC_IMM`), not as `SRC_REG` (see Register format).

## Storage field

This field is optional.

The field ` -> c_storage` specifies how the value of the register `c` after executing the operation must be stored.  

The possible formats of `c_storage` are:
- `rN`, meaning that the current value of register `c` will be stored into register `rN`.  The ZiskInst instance field `store` is set to STORE_REG, and the field `store_offset` is set to N.
- `[N]`, meaning that register `c` will be stored in memory at address N, plus the SP register if specified.  The ZiskInst instance field `store` is set to STORE_MEM, and the field `store_offset` is set to N.
- `W[a+N]`, meaning that the first W bytes of the current value of register `c` will be stored at the first W bytes in memory at address equals the value of `a` register plus the value of N, which can be a negative offset.  The ZiskInst instance field `store` is set to STORE_IND, and the ZiskInst instance field `store_offset` is set to the address offset, i.e. to N including sign, and the field `ind_width` is set to W, which can be 1, 2, 4 or 8.
- If not specified, the `c` register is not stored.  The ZiskInst instance field `store` is set to STORE_NONE.

## Jump field

This field is optional.

The field `, j(jump1, jump2)` specifies how the program counter (`pc`) must be updated after the execution of the operation in order to jump to the next instruction.

The fields `jump1` and `jump2` can be either a signed integer number (i64), in which case they refer to the offset of the target instruction to jump to vs. the current pc, or they can be an instruction label.

If after the operation execution the flag register equals 1, then the next instruction is the one referred by `jump1`.  If the flag register equals 0, then the next instruction is the one referred by `jump2`.

When `jump2` refers to the next instruction (i.e. current_pc + 4) then it can be omitted and this field can be simplified to `, j(jump1)`;

When the next instruction to execute is always current_pc + 4, this field can be omitted.

## Set PC field

This field is optional.

The field `, setpc(offset)` specifies what the next instruction pc will be, i.e. what the next instruction to execute is.  The ZiskInst instance field `set_pc` is set to true.

The field `offset` is a signed integer (i64), and is stored in the ZiskInst instance field `jump_offset1`.  The next instruction pc will be the `c` register value plus the `offset` field value.

In other words:

```
next_instruction_pc = c + offset
```

Note that `setpc` and the flag-based jump `j(jump1, jump2)` are mutually exclusive on the same instruction: both are stored in the field `jmp_offset1`, and the emulator checks `set_pc` first.  If `set_pc` is true, the next pc is `c + jmp_offset1` and the `flag` register is ignored; otherwise the next pc is `pc + jmp_offset1` when `flag` is 1, or `pc + jmp_offset2` when `flag` is 0.

## SP field

This field is optional.

The field `, sp` specifies that the `sp` register must be added to the address of the memory-addressed operands of the instruction.

There is no single `use_sp` field; this sets the per-operand flags `a_use_sp_imm1`, `b_use_sp_imm1` and `store_use_sp` of the ZiskInst instance, as applicable.  It only affects memory-addressed operands, i.e. the `[N]` (`SRC_MEM`) and `W[a+N]` (`SRC_IND`) sources, and the `[N]`/`W[a+N]` stores; it has no effect on `c`, `rN`, immediate (`N`) or `step` operands.

Note that for an immediate source (`N`), the same field (`a_use_sp_imm1` / `b_use_sp_imm1`) holds the higher 32 bits of the immediate value, so `sp` cannot be combined with an immediate on the same operand.

## End field

This field is optional.

The field `end` specifies that this is the last instruction of the program to be executed.

This field sets the field `end` of the ZiskInst instance to true.

## Pseudo-instructions

Pseudo-instructions are convenience mnemonics that the assembler expands into one or more ZisK instructions.

`call` and `ret` provide function-call semantics equivalent to RISC-V, using register `r1` as the return-address register (the RISC-V `ra`). `push`/`pop` provide a software stack on `sp` (`r2`) for saving `r1` (and any live values) across nested calls.

### call

```
call LABEL
```

The `call` pseudo-instruction stores the return address (the address of the instruction that follows the `call`) into register `r1`, and then jumps to `LABEL`.

It assembles to a single ZisK instruction that:
- uses the `flag` operation, which sets the `flag` register to 1;
- sets the ZiskInst `store_pc` flag, with `store` set to STORE_REG and `store_offset` set to 1, so that the instruction stores the value `pc + jmp_offset2` (the return address) into register `r1` instead of storing the `c` register;
- sets `jmp_offset1` to the offset from the current instruction to `LABEL`, and `jmp_offset2` to the instruction size (4).

Because `flag` is 1, the next pc is `pc + jmp_offset1` (i.e. `LABEL`), while the value stored into `r1` is `pc + jmp_offset2` (i.e. the address of the next instruction).  This is the equivalent of the RISC-V `jal r1, LABEL`.

### ret

```
ret
```

The `ret` pseudo-instruction jumps to the address held in register `r1`.

It assembles to a single ZisK instruction, equivalent to writing:

```
and(0xfffffffffffffffe, r1), setpc(0)
```

The `and` masks off bit 0 of `r1` (the RISC-V JALR target-alignment rule), producing the target address in the `c` register, and `setpc(0)` sets the next pc to `c`, i.e. to `r1 & ~1`.  No value is stored.  This is the equivalent of the RISC-V `ret` = `jalr r0, r1, 0`.

### jump

```
jump(TARGET)
```

The `jump` pseudo-instruction performs an unconditional *static* jump to `TARGET`, which is either a label or an absolute address (a number).  Note that, unlike the `j(...)` field and `call`, a numeric `jump` target is an absolute address, not a pc-relative offset.

It assembles to a single ZisK instruction equivalent to:

```
copyb(0, TARGET_ADDRESS), setpc(0)
```

i.e. it loads the target address as a constant into the `c` register and sets the next pc to `c`.  Because the target is a constant, the x86 assembly generator compiles it to a direct jump, which — unlike a register-based dynamic jump such as `ret` — works for any address, including low (`< ROM_ADDR`) BIOS addresses.

### ret_to_bios

```
ret_to_bios
```

The `ret_to_bios` pseudo-instruction returns control to the ZisK BIOS finalization code, which reads the program output from `OUTPUT_ADDR` and ends the program.  The BIOS entered the program (its `_start`) leaving this return address in `r1`, so `ret_to_bios` jumps there.

It assembles to a static `jump` to the BIOS finalization address, which the assembler derives from the BIOS layout (it is not hard-coded).  A dynamic `ret` cannot be used here: the return address is a low (`< ROM_ADDR`) address, and the x86 assembly generator's dynamic-jump path assumes high addresses.

### push / pop

```
push rN
pop rN
```

`push`/`pop` maintain a downward-growing software stack on the stack pointer `sp` (`r2`), which the launcher (and, for a redirected library routine, the guest) initialises to a valid stack region.  Each expands to **two** ZisK instructions:

- `push rN` → `sub(r2, 8) -> r2` then `copyb(r2, rN) -> 8[a + 0]` (decrement `sp`, store `rN` at `[sp]`).
- `pop rN` → `copyb(r2, 8[a + 0]) -> rN` then `add(r2, 8) -> r2` (load `rN` from `[sp]`, increment `sp`).

A label preceding a `push`/`pop` binds to its first instruction.  These are the idiom for a non-leaf routine to preserve the return address across a nested `call` (`call` overwrites `r1`), and to spill any values that must survive a call — ZisK has no other stack mechanism.  Typical prologue/epilogue:

```
myfunc:
    push r1          ; save return address
    ; ... body, may `call` other routines ...
    pop r1           ; restore it
    ret
```

## Program entry and automatic launcher

The program entry point is the `_start` label, which the assembler places at `ROM_ADDR` (the ELF convention that the entry point is the program base).  Source files may be supplied in any order — a `-z <dir>` run collects every `.zisk` file in the directory *and its subdirectories* (recursively), sorted by path — and the assembler moves the file that defines `_start` first.

If a program does not define `_start`, the assembler synthesizes a launcher automatically around the program's entry label — `main`, or otherwise `_zisk_main` — mirroring `ziskos::_start`:

```
_start:
    copyb(0, 0) -> r3            ; gp = _global_pointer
    copyb(0, 0xa0400000) -> r2   ; sp = _init_stack_top (SYS_ADDR)
    call main                    ; or _zisk_main
    ret_to_bios
```

This lets a program be just its own code plus a `main:` label, with no hand-written boot file.  Compare `examples/doubler` (explicit launcher in `ziskos.zisk`) with `examples/doubler-min` (automatic launcher).

<!--

TODO:

[Optimization] Logical instruction size:
    bits between 2 consecutive addresses
    currently 32 bits = 4 bytes
    RISC-V uses ROM_SIZE (128M) / 4 = 32M instructions -> we could expand it to 128M instructions

Calling convention: which registers a callee must preserve across a call (ideally, only those it uses)

How to split code between different files:
    imports or includes of other files
    external functions and data
    ”Makefile” with a list of files to compile…
    currently: -z file.zisk, or -z folder -> folder/*.zisk

How to integrate BIOS code, e.g. how to do syscall

How to call read_input / write_output / simply access the input/output memory addresses

Hints / pragmas

CLI integration: be able to generate a proof
    ziskemu -z file.zisk ..
    zisk2zisk -> emu.asm
    ziskemuasm -z ..
    cargo-zisk prove
    .zisk -> compile -> ZiskRom -> save_as_asm() -> .asm

Define macros with parameters and multi-line code, e.g. define my_macro(a,b,c) ...

Import a file.elf (e.g. libfloat.elf)

Start working on the Eth client

Call a function with parameters:
call my_func(a, b, c, d, e) -> save a into a0, b into a1... call my_func

-->