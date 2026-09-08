# ziskbin — binary encoding of a `ZiskRom` (for embedding in an ELF)

`ziskbin` is a compact, variable-length serialization of a fully-built
[`ZiskRom`](../core/src/zisk_rom.rs). It lets a `.zisk` program (or any already-built
ROM) be stored in an ELF container and decoded back into an identical `ZiskRom`,
bypassing RISC-V transpilation — so the existing ELF-based toolchain
(`cargo-zisk prove`, `execute`, ROM setup) works unchanged.

Guiding principle: **encode only what carries information.** Fields that are
always default, derivable from the opcode, or re-derivable from the ROM layout
are not stored. A typical instruction costs a handful of bytes instead of its
~200-byte in-memory footprint.

## Layering

```
┌─ ELF container ────────────────────────────────────────────┐  §2
│  minimal ELF header + marker + one section holding:         │
│  ┌─ ROM container ──────────────────────────────────────┐  │  §3
│  │  header  (magic, version, profile, base, counts)      │  │
│  │  instruction stream   (addr-delta + instruction) ×N   │  │  §3.2, §5
│  │  RO data sections     (DataSection64) ×R              │  │  §4
│  │  RW data sections     (DataSection64) ×W              │  │  §4
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

The ROM container (§3–§6) is transport-agnostic; the ELF framing (§2) is just the
current delivery vehicle.

---

## 2. ELF framing

The blob is delivered inside an otherwise-minimal ELF so that `is_elf_file`
([core/src/utils.rs](../core/src/utils.rs)) and the pipeline's ELF guards accept the
file, while the transpiler recognizes it and loads the ROM directly.

- **Marker.** A ziskbin ELF self-identifies via a private `e_machine` value
  (`EM_ZISK`, chosen from the ELF reserved-for-vendor range) and/or a dedicated
  section named `.ziskrom` (`SHT_PROGBITS`). Standard ELF tools ignore an unknown
  machine and unknown sections.
- **Payload.** The `.ziskrom` section contains exactly the ROM container of §3.
- **Header.** A valid `EI_MAG`/`EI_CLASS`/`EI_DATA`/`EI_VERSION` `e_ident`, a
  nonzero `e_entry` (any value; the real entry is inside the instruction stream —
  see §3.1), and one section header for `.ziskrom`. Nothing else is required.
- **Detection & branch.** At the top of `elf2rom`
  ([transpilers/common/src/elf2rom.rs](../transpilers/common/src/elf2rom.rs)) — before the
  RISC-V `e_entry`/payload logic — check for the marker. If present, read
  `.ziskrom` and hand it to the ROM-container decoder (§3), returning the
  `ZiskRom`. Otherwise fall through to the existing RISC-V path. Every caller
  (`Riscv2zisk::run`/`runfile`, `CustomRom::parse_rom`) inherits this for free.

---

## 3. ROM container

```
container := header  instruction_stream  ro_sections  rw_sections
```

### 3.1 Header

| Field | Encoding | Meaning |
|-------|----------|---------|
| `magic` | 4 bytes `"ZKRM"` | Sanity/format tag |
| `version` | `u8` | Container format version (§8) |
| `profile` | `u8` | bit0 = *stripped* (drop debug fields, §7); other bits reserved |
| `inst_count` | uvarint | Number of instructions `N` |
| `ro_count` | uvarint | Number of RO data sections |
| `rw_count` | uvarint | Number of RW data sections |

No entry-point, PC bounds, or lookup tables are stored: the entry is realized as
instructions in the `ROM_ENTRY` (`0x1000`) region (the BIOS entry jump produced by
`add_entry_exit_jmp`), and all PC bounds / lookup vectors are re-derived (§6). The
emulator always boots at `ROM_ENTRY`.

### 3.2 Instruction stream

`N` records in **strictly ascending address order**. Each record:

```
record := addr_delta(uvarint)  instruction(§5)
```

`addr_delta` is the gap from the previous instruction's address (with a running
cursor initialized to `0`, so the first record's delta is the first absolute
address). Addresses are unique and ascending, so every delta ≥ 1. This supplies
each instruction's `paddr` (class **C** in §5) without repeating a full address.

See §3.3 for why deltas — not positional/0-filled layout — are the right model
for mixed-source ROMs.

### 3.3 Address model — mixed sources, arbitrary alignment

A `ZiskRom` is **not** uniformly 4-byte-spaced. `optimize_instruction_lookup`
([zisk_rom.rs](../core/src/zisk_rom.rs)) sorts instructions into five categories, three of
which are 4-byte-aligned (BIOS `[ROM_ENTRY,ROM_ADDR)`, main `[ROM_ADDR,
FLOAT_LIB_ROM_ADDR)`, float `[FLOAT_LIB_ROM_ADDR, ROM_ADDR_MAX]`) and **two of
which are byte-granular** ("non-aligned program/float", accessed as
`array[addr - base]`). RISC-V transpilation places **internal split
instructions** at intermediate offsets (`last_internal_address_offset` = 1, 3,
5 … from the base), and compressed RISC-V is 2-byte aligned. Mixing lib-float
(a RISC-V ELF), `.zisk` code, and other sources therefore yields instructions at
**arbitrary byte addresses** with **arbitrary gaps**.

The delta encoding above handles this directly: `+4` for aligned `.zisk`/RISC-V,
`+2` for compressed, `+1/+2` for internal splits, and one large varint for a
region jump (e.g. `ROM_ENTRY → ROM_ADDR`, which occurs a handful of times).
Common deltas cost **one byte**.

**Why not fill the unused positions with a `0` byte** (a positional, address =
offset layout)? Three problems:

1. **Records are variable-length.** An instruction at address `A` occupies 3–20+
   bytes, so it already overruns the byte positions of `A+1, A+2, …`. Physical
   offset in the blob cannot equal the logical ROM address, so there are no fixed
   "slots" to pad.
2. **`0x00` is a real opcode** (`flag`, [zisk_ops.rs](../core/src/zisk_ops.rs)), so a bare
   `0x00` cannot unambiguously mean "empty address" — it collides with a genuine
   instruction start.
3. **The gaps are not empty in general.** For RISC-V-derived code the `A+1/A+2/A+3`
   positions hold the internal split instructions; only pure `.zisk` leaves them
   unused. Padding them would both waste 3 bytes per `.zisk` instruction and be
   wrong wherever those addresses are occupied.

A delta encodes an arbitrarily wide gap in one small varint, so it strictly
dominates 0-filling on both generality and size.

---

## 4. RO / RW data sections

RO data (`ro_data_64`) becomes ELF-region read-only memory; RW data
(`rw_data_64`) is initialized RAM. Each is a `Vec<DataSection64>`, and
`DataSection64` is `{ addr: u64, data: Vec<u64> }` — a base address plus a run of
8-byte words placed at `addr + 8*i`.

Per section:

```
section := addr(uvarint)  word_count(uvarint)  word[word_count]
word     := 8 raw little-endian bytes        # arbitrary 64-bit values
```

`ro_count`/`rw_count` sections follow the instruction stream, RO first then RW,
each block in the container's section order.

Notes:
- Sections are stored **verbatim** — this is real content, not derivable. The
  transpiler's zero-run carving / normalization
  (`normalize_rw_data_sections`) is a *build-time* concern; the container simply
  serializes whatever sections the built ROM contains.
- **Provability constraint:** each section's `word_count` must be a multiple of 4
  (32 bytes) and its `addr` 32-byte aligned. The ROM-init trace commits data in
  4-u64 rows anchored at `addr` (`state-machines/rom/src/custom_rom.rs`), so a
  non-conforming section is rejected during proving (though the emulator accepts
  it). Producers must pad/align: the RISC-V transpiler does via `RO_SECTION_ALIGN`,
  and the ziskasm assembler pads both sections and 32-byte-aligns the ROM data base.
- `word` uses raw 8 bytes (not varint): data words are arbitrary and often large,
  and large all-zero runs are already carved out upstream. (A future version may
  add a varint-per-word or RLE profile bit if a workload benefits.)

---

## 5. Instruction codec

One `ZiskInst` is `op` + a flag bitmap + payloads for the non-default fields.

### 5.1 Field classification

| Class | Meaning | Encoded? |
|-------|---------|----------|
| **M** — Mandatory | Always present; defines the instruction | Always |
| **V** — Variable | Independent value; emitted iff ≠ canonical default | Conditionally |
| **D** — Derived | A pure function of the opcode; never stored | Never (reconstructed) |
| **C** — Container | Supplied by the address stream (§3.2) | Not in the inst body |

| # | Field | Type | `Default` | Canonical default | Class |
|---|-------|------|-----------|-------------------|-------|
| 1 | `op` | `u8` | `0` | — | **M** |
| 2 | `a_src` | `u64` | `0` | `0` (`SRC_C`) | V |
| 3 | `a_offset_imm0` | `u64` | `0` | `0` | V |
| 4 | `a_use_sp_imm1` | `u64` | `0` | `0` | V |
| 5 | `b_src` | `u64` | `0` | `0` (`SRC_C`) | V |
| 6 | `b_offset_imm0` | `u64` | `0` | `0` | V |
| 7 | `b_use_sp_imm1` | `u64` | `0` | `0` | V |
| 8 | `store` | `u64` | `0` | `0` (`STORE_NONE`) | V |
| 9 | `store_offset` | `i64` | `0` | `0` | V |
| 10 | `store_pc` | `bool` | `false` | `false` | V |
| 11 | `store_use_sp` | `bool` | `false` | `false` | V |
| 12 | `set_pc` | `bool` | `false` | `false` | V |
| 13 | `ind_width` | `u64` | `0` | `0` | V |
| 14 | `end` | `bool` | `false` | `false` | V |
| 15 | `jmp_offset1` | `i64` | `0` | **`INST_SIZE` (4)** | V |
| 16 | `jmp_offset2` | `i64` | `0` | **`INST_SIZE` (4)** | V |
| 17 | `verbose` | `String` | `""` | `""` | V (debug) |
| 18 | `riscv_inst` | `Option<String>` | `None` | `None` | V (debug) |
| 19 | `index` | `u64` | `0` | `0` | V |
| 20 | `sorted_pc_list_index` | `usize` | `0` | `0` | V/L (§6) |
| 21 | `next_internal_inst` | `Option<u64>` | `None` | `None` | V |
| 22 | `external_ref_addr` | `Option<u64>` | `None` | `None` | V |
| 23 | `meta_rs1` | `Option<u8>` | `None` | `None` | V |
| 24 | `meta_rd` | `Option<u8>` | `None` | `None` | V |
| 25 | `paddr` | `u64` | `0` | — | **C** (§3.2) |
| 26 | `func` | `fn(&mut InstContext)` | `\|_\| ()` | — | **D** = `ZiskOp::try_from_code(op).get_call_function()` |
| 27 | `op_str` | `&'static str` | `""` | — | **D** = `ZiskOp…name()` |
| 28 | `op_type` | `ZiskOperationType` | `None` | — | **D** = `ZiskOp…op_type()` |
| 29 | `is_external_op` | `bool` | `false` | — | **D** = `op_type ∉ {Internal, Fcall}` |
| 30 | `is_precompiled` | `bool` | `false` | — | **D** = `input_size > 0` |
| 31 | `input_size` | `u64` | `0` | — | **D** = `ZiskOp…input_size()` |
| 32 | `m32` | `bool` | `false` | — | **D** = `ZiskOp…name().contains("_w")` |

The **D** fields — a function pointer, three opcode-metadata copies, two booleans,
and a `u64` — are a pure function of `op` and cost **zero** bytes; they are
recomputed on decode via [`ZiskOp::try_from_code`](../core/src/zisk_ops.rs).

### 5.2 Primitive encodings

| Primitive | Encoding |
|-----------|----------|
| `u8` | one raw byte |
| unsigned varint | **LEB128**, little-endian, high bit = continuation |
| signed varint | **zig-zag** (`(n<<1) ^ (n>>63)`) then LEB128 |
| `bool` | no payload — the field's flag bit **is** the value |
| `String` | LEB128 length, then that many UTF-8 bytes |
| `Option<T>` | flag clear ⇒ `None`; flag set ⇒ `Some`, then `T`'s payload |

### 5.3 Canonical defaults

A V-field is omitted when it equals its **canonical default** — usually the Rust
`Default` (`0`/`false`/`None`/`""`), with the deliberate exception of
`jmp_offset1`/`jmp_offset2`, whose canonical default is **`INST_SIZE` (4)**. The
assembler emits `j(INST_SIZE, INST_SIZE)` for every straight-line instruction
([assembler.rs](src/assembler.rs)) and the emulator advances `pc += jmp_offset2`
when the flag is false (`+= jmp_offset1` when true without `set_pc`). Anchoring
the default at 4 makes the common case emit **zero** jump bytes; only real
branches/calls pay.

### 5.4 Wire format

```
+--------+===============+========================+
|  op    |   flag bytes  |   field payloads       |
| (u8)   | (1..N: 7 bits | (present V-fields, in  |
|        |  + cont bit)  |  ascending flag order) |
+--------+===============+========================+
```

Each flag byte carries **7 field-flags** (bits 0–6) + a **continuation bit**
(bit 7). Flag byte *k* covers flags `7k..7k+6`; trailing all-zero flag bytes are
not emitted. Payloads follow in ascending flag order; booleans and `None` options
contribute a flag bit but no payload.

| Flag | Field | Payload | | Flag | Field | Payload |
|------|-------|---------|---|------|-------|---------|
| **f0** | `a_src` | uvarint | | **f12** | `ind_width` | uvarint |
| f1 | `a_offset_imm0` | uvarint | | f13 | `jmp_offset1` | svarint |
| f2 | `a_use_sp_imm1` | uvarint | | **f14** | `jmp_offset2` | svarint |
| f3 | `b_src` | uvarint | | f15 | `verbose` | string |
| f4 | `b_offset_imm0` | uvarint | | f16 | `riscv_inst` | string (⇒`Some`) |
| f5 | `b_use_sp_imm1` | uvarint | | f17 | `index` | uvarint |
| f6 | `store` | uvarint | | f18 | `sorted_pc_list_index` | uvarint |
| **f7** | `store_offset` | svarint | | f19 | `next_internal_inst` | uvarint (⇒`Some`) |
| f8 | `store_pc` | — bool | | f20 | `external_ref_addr` | uvarint (⇒`Some`) |
| f9 | `store_use_sp` | — bool | | **f21** | `meta_rs1` | u8 (⇒`Some`) |
| f10 | `set_pc` | — bool | | f22 | `meta_rd` | u8 (⇒`Some`) |
| f11 | `end` | — bool | | f23–f27 | *reserved* | — |

(**bold** = first flag of each new flag byte.) Behavioural fields cluster in
bytes 0–1, so most instructions need only 1–2 flag bytes; bytes 2–3 (debug /
callstack metadata) are absent for a stripped, container-addressed ROM. `paddr`
is class **C** (§3.2), so no flag bit is needed for it in a ROM.

### 5.5 Encode / decode

```text
encode_inst(inst) -> bytes:
    out = [inst.op]; flags = 0; payload = []
    for each V-field f in flag order:
        if f is bool:          if inst.f: set(flags, f)
        elif f is Option:      if inst.f is Some(v): set(flags, f); payload += enc(v)
        else:                  if inst.f != canonical(f): set(flags, f); payload += enc(inst.f)
    out += emit_bitmap(flags)   # 7 bits/byte + continuation; drop trailing 0 bytes
    return out + payload

decode_inst(reader, addr_from_stream) -> ZiskInst:
    op = reader.u8(); flags = read_bitmap(reader)
    inst = ZiskInst::default(); inst.op = op
    z = ZiskOp::try_from_code(op)?                         # (1) reconstruct D fields
    inst.func = z.get_call_function(); inst.op_str = z.name()
    inst.op_type = z.op_type().into(); inst.input_size = z.input_size()
    inst.is_precompiled = inst.input_size > 0
    inst.is_external_op = inst.op_type not in {Internal, Fcall}
    inst.m32 = z.name().contains("_w")
    inst.jmp_offset1 = INST_SIZE; inst.jmp_offset2 = INST_SIZE   # (2) non-zero canonical defaults
    for each V-field f in flag order where set(flags, f):        # (3) overlay present fields
        inst.f = (f is bool) ? true : dec_payload(reader, f)
    inst.paddr = addr_from_stream                               # (4) address from §3.2
    return inst
```

Order matters: **derive from op → apply canonical defaults → overlay present
fields → set `paddr`**.

---

## 6. Layout reconstruction

After decoding all instructions and data sections, rebuild everything the ROM
derives — do **not** store it:

- Insert instructions into `insts` keyed by `paddr`, then run
  `optimize_instruction_lookup` to regenerate `rom_bios_instructions`,
  `rom_program_instructions`, `rom_program_na_instructions`,
  `rom_float_instructions`, `rom_float_na_instructions`, `sorted_pc_list`,
  `sorted_pc_list_index` (per inst), and `max_bios_pc` / `max_program_pc` /
  `max_float_pc` / `min_program_pc`.
- `build_counter`, `last_internal_address_offset`, `next_init_inst_addr` are
  build-time cursors — reset/recompute; not serialized.

`sorted_pc_list_index` (f18) and `index` (f17) therefore have escape-hatch flags
only for the rare case a caller must pin an exact original layout; normally both
are left unset and re-derived.

---

## 7. Stripped vs. faithful profiles

Selected by header `profile` bit0:

- **Faithful** — encode every differing V-field (incl. `verbose`, `riscv_inst`,
  `index`, callstack metadata). Round-trips byte-identically (given addresses).
- **Stripped** (recommended for proving) — drop `verbose` (f15) and `riscv_inst`
  (f16); let layout fields re-derive. These do not affect execution or the proof,
  and `verbose` is the only large per-instruction payload.

---

## 8. Versioning & forward compatibility

- The container `version` byte (§3.1) governs the whole format.
- New `ZiskInst` fields take the next free flag bit (f23…). A decoder cannot skip
  an unknown payload-bearing flag, so **assigning a new payload flag requires a
  version bump**. Reserved bits f23–f27 (and further continuation bytes) leave
  room to grow.
- Reordering flag bits, changing a canonical default, or changing the header/
  section layout is a breaking change — bump `version` and record it here.
