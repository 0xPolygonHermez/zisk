# auipc_rv64_overflow - RV64 AUIPC across the 32-bit limb boundary

Two tests, same instruction (`auipc a1, 0x57fff`), different instruction
addresses.  The only relevant difference is whether `pc + imm` crosses the
32-bit limb boundary.

```
auipc_rv64_safe/     auipc at PC = 0xA8000000  ->  a1 = 0xFFFFF000   (high limb 0)
auipc_rv64_overflow/ auipc at PC = 0xA8001000  ->  a1 = 0x100000000  (high limb 1)
```

**Both are proven.** These fixtures guard that: an AUIPC whose result is at or
above 2^32 is valid RV64 and is what a guest runs to take the address of static
data placed above `0x1_0000_0000`, now that the RAM ends at `0x1_4000_0000`.

The overflow case uses `.balign 4096` in the assembly to place `auipc_test` at a
4 KB-aligned address, mirroring what happens when a linker places a section at the
next page boundary. Both tests write the result to `a1` and exit with code 0: a
program that exits with a non-zero `a0` is rejected by the prover for that reason
alone.

## The arithmetic

```
auipc a1, 0x57fff
  offset = 0x57FFF << 12 = 0x57FFF000   (positive 20-bit AUIPC immediate, bit 19 = 0)

At PC = 0xA8000000:  0xA8000000 + 0x57FFF000 = 0xFFFFF000          -> high limb 0
At PC = 0xA8001000:  0xA8001000 + 0x57FFF000 = 0x100000000 = 2^32  -> high limb 1
```

## Why it works, and the limitation behind the name

The Main/Mem bus represents register and memory values as two 32-bit limbs.

The RISC-V transpiler turns an AUIPC into a `copyb` of the constant `pc + imm`
(on its own, or fused with a following `jalr` or `addi`). An immediate carries
both limbs, so `0x100000000` reaches the bus as `[0, 1]`.

`main.pil` also has a `store_pc` path that stores `pc + jmp_offset2` as a single
limb:

```
store_value[0] = pc + jmp_offset2
store_value[1] = 0                   // carry out of bit 31 is dropped
```

A result at or above 2^32 would not be provable through that path. It is not
reachable: `store_pc` is only used to save a return address (`pc + 4` of
`jal`/`jalr`, and of the WASM calls), which is a ROM address below 2^32. AUIPC
does not use it. Any new use of `store_pc` must keep its result below 2^32.

## Running it

From the repository root:

```
./elf-regressions/auipc_rv64_overflow/demo-auipc-rom-setup
```

It uses the helpers next to it: `build-elf` assembles a `.s` file into an ELF,
`rom-setup-elf` runs `cargo-zisk program-setup` on it, and `prove-elf` runs
`cargo-zisk prove` on it and verifies the proof. ROM setup and proving are both
expected to succeed.
