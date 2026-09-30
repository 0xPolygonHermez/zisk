# Core Audit draft-report by [ZippelLabs](https://zippellabs.github.io/)


## [MEDIUM-1] Unbounded memory allocation from ELF sections

**Context**: `core/src/elf_extraction.rs`

### Description

The code in `collect_elf_payload_from_bytes` allocates memory for ELF sections based solely on the section header sizes without any upper bound. 
A malicious or corrupted ELF file can specify extremely large SHT_PROGBITS or SHT_NOBITS section sizes, causing `raw.to_vec()` or `vec![0u8; aligned_size]` to allocate excessive memory. This can lead to out-of-memory conditions or process crashes, enabling a Denial-of-Service attack when parsing untrusted ELF files.

### Recommendation 

Validate and enforce reasonable limits on section sizes before allocating memory. For example, ensure that `sh.sh_size` does not exceed a configurable maximum (such as the expected binary size or available RAM), and return an error if the size is too large.

### Impacted Code

```rust
if sh.sh_type == SHT_PROGBITS {
    let (raw, _) = elf.section_data(&sh)?;
    let mut data = raw.to_vec();
    // Word-align by trimming
    while data.len() % 4 != 0 {
        data.pop();
    }
    data
} else if sh.sh_type == SHT_NOBITS {
    // BSS sections - uninitialized data, should be zero-filled
    // Create a zero-filled vector of the appropriate size
    let size = sh.sh_size as usize;
    // Align size to 4 bytes
    let aligned_size = (size + 3) & !3;
    vec![0u8; aligned_size]
} else {
    // Skip other section types (notes, etc.)
    continue;
}
```



## [MEDIUM-2]Unhandled RISC-V instruction causes panic

**Context**: `core/src/riscv2zisk_context.rs`

### Description

In `Riscv2ZiskContext::convert()`, the match on riscv_instruction.inst ends with a panic for any unrecognized instruction. If the translator encounters an unexpected or maliciously crafted RISC-V instruction mnemonic, it will panic and crash the entire transpilation process, leading to a denial of service.

### Recommendation

Replace the panic in the default arm with error handling. For example, return a Result or skip unsupported instructions instead of aborting. Log the unsupported mnemonic and continue or propagate an error.

### Impacted Code

```
_ => panic!(
    "Riscv2ZiskContext::convert() found invalid riscv_instruction.inst={}",
    riscv_instruction.inst
),
```

## [LOW-1]Missing output argument causes panic on unwrap

**Context**: `core/src/bin/riscv2zisk.rs`

### Description

The CLI accepts 3 arguments (program name + 2 params), but when only 2 params are provided it sets `asm_file` to `None` and later unconditionally calls `asm_file.unwrap()`, which will panic. If this binary is invoked with user-controlled arguments (e.g., by a service/wrapper), an attacker can reliably trigger a crash (denial of service) by omitting the optional asm/output parameter.

### Recommendation 

Do not unwrap `asm_file`. Either (a) require the asm/output file argument by enforcing `args.len() == 4`, or (b) provide a default output path when `args.len() == 3`, or (c) handle `None` with a clear error and exit code without panicking.
Impacted Code :
```rust
let (asm_file, gen_arg) = if args.len() == 4 {
    (Some(args[2].clone()), args[3].clone())
} else {
    (None, args[2].clone())
};
...
if let Err(e) = rv2zk.runfile(asm_file.unwrap(), generation_method, true, true) {
```


#### Note: During auditing `core/scr` - 1-H, 3-M, and 2-L, bugs has been found. 

