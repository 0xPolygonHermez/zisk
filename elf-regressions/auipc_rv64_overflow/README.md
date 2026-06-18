# auipc_rv64_overflow — RV64 AUIPC circuit bug

Two tests, same instruction (`auipc a0, 0x7ffff`), same standard Zisk linker script.
The only difference is where in ROM the instruction executes.

```
auipc_rv64_safe/     auipc at PC = 0x80000000   →   a0 = 0xFFFFFF00   (fits 32 bits, proving OK)
auipc_rv64_overflow/ auipc at PC = 0x80001000   →   a0 = 0x100000000  (= 2^32, proving FAILS)
```

The overflow case uses `.balign 4096` in the assembly to place `auipc_test` at a
4 KB-aligned address, mirroring what happens when a linker places a section at the
next page boundary.

## The arithmetic

```
auipc a0, 0x7ffff
  offset = 0x7FFFF << 12 = 0x7FFFF000   (largest positive 20-bit AUIPC immediate, bit 19 = 0)

At PC = 0x80000000:  0x80000000 + 0x7FFFF000 = 0xFFFFFF00          ← fits in 32 bits ✓
At PC = 0x80001000:  0x80001000 + 0x7FFFF000 = 0x100000000 = 2^32  ← does not fit ✗
```

## Why proving fails

`mem.pil` constrains every stored register value to 32 bits:

```
col witness bits(32) air.value[RC];
value[i] = value_word[i*2] + 65536 * value_word[i*2+1]
range_check(value_word[...], 0, 65535)   // two 16-bit halves → max 2^32 - 1
```

`main.pil` computes the value to store for AUIPC:

```
store_value[0] = pc + jmp_offset2    // = PC + (imm << 12)
store_value[1] = 0                   // carry out of bit 31 is hardcoded to zero
```

When `store_value[0] = 0x100000000 > 2^32 − 1`, no valid decomposition into two
16-bit halves exists.  The constraint is unsatisfiable and `VerifyGlobalConstraints`
fails.

## Why this is a spec violation

RV64I Unprivileged Spec §2.4:

> "AUIPC … sign-extends the result to 64 bits, adds it to the address of the
> AUIPC instruction, then places the result in register rd."

The result is a **64-bit value**.  It is not required to fit in 32 bits.
`store_value[1]` must receive the carry out of bit 31 rather than being
hardcoded to zero.
