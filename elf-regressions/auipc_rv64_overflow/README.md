# auipc_rv64_overflow - RV64 AUIPC circuit bug

Two tests, same instruction (`auipc a0, 0x7ffff`), different instruction
addresses.  The only relevant difference is whether `pc + imm` crosses the
32-bit limb boundary.

```
auipc_rv64_safe/     auipc at PC = 0x80000000  ->  a0 = 0xFFFFFF00   (high limb 0, proving OK)
auipc_rv64_overflow/ auipc at PC = 0x80001000  ->  a0 = 0x100000000  (high limb 1, proving FAILS)
```

The overflow case uses `.balign 4096` in the assembly to place `auipc_test` at a
4 KB-aligned address, mirroring what happens when a linker places a section at the
next page boundary.

## The arithmetic

```
auipc a0, 0x7ffff
  offset = 0x7FFFF << 12 = 0x7FFFF000   (largest positive 20-bit AUIPC immediate, bit 19 = 0)

At PC = 0x80000000:  0x80000000 + 0x7FFFF000 = 0xFFFFFF00          -> high limb 0
At PC = 0x80001000:  0x80001000 + 0x7FFFF000 = 0x100000000 = 2^32  -> high limb 1
```

## Why proving fails

The Main/Mem bus represents register and memory values as two 32-bit limbs.
`main.pil` computes the value to store for AUIPC as:

```
store_value[0] = pc + jmp_offset2    // = PC + (imm << 12)
store_value[1] = 0                   // carry out of bit 31 is hardcoded to zero
```

For `0x100000000`, the correct two-limb representation is `[0, 1]`.  The
current `store_pc` expression instead supplies `[0x100000000, 0]`, which does
not match the 64-bit value emitted by the executor/transpiler path and causes
`VerifyGlobalConstraints` to fail.

## ROM setup check

Run this helper to build the overflow fixture and demonstrate that ROM setup
accepts it:

```
./demo-auipc-rom-setup
```

The expected result is a zero exit with `ROM setup successfully completed`.
This shows the failure is not caught while decoding/transpiling the ELF or
generating ROM setup artifacts.

## Why this is a spec violation

RV64 AUIPC writes an XLEN-wide `pc + sext(imm20 << 12)` result to `rd`.
On RV64, that result is not required to fit in 32 bits.  `store_value[1]`
must receive the high limb of the 64-bit result rather than being hardcoded
to zero.
