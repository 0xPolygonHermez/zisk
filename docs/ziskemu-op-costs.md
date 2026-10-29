# ziskemu opcode costs

Source: `SUMMARY` lines of `build/setup.log` (*Total* column) and the instances in `pil/zisk.pil`.

Rule: cost per operation = ⌈Total / ops_per_row⌉ × rows_per_op, where *ops_per_row* is
`lanes_x_row` / `op_x_row` / `LANES`. When several instances can prove an opcode, **Current** is the
cheapest one and **Max** the most expensive one among those able to prove it.

Constants live in `core/src/zisk_ops_costs.rs` (opcodes) and `emulator/src/emu_costs.rs` (memory,
main, base).

Cost model version: `ZISK_COSTS_VERSION = 1`. It must be increased whenever these costs, the way
they are accumulated, or the FROPS change.

## Main and standard operations

| Cost | Opcodes | Instances (Total / ops x row → cost) | Previous | Current | Max |
|---|---|---|---:|---:|---:|
| `MAIN_COST` (per step) | all | Main 245/4 → 62 | 68 | **62** | 62 |
| `BINARY_COST` | minu, min, maxu, max, ltu, lt, eq, sub, leu, le, and, or, xor, *_w, brev8, andn, orn, xnor, sh1add, sh2add, sh3add, unal8 | Binary 60/1 → 60, BinaryLarge 114/2 → 57, BinaryHuge 219/4 → 55 | 60 | **55** | 60 |
| `BINARY_ADD_COST` | add | BinaryAdd 26/1 → 26, BinaryAddLarge 46/2 → 23, BinaryAddHuge 83/4 → 21, Binary* 55–60 | 25 | **21** | 60 |
| `BINARY_ADD_HI_COST` | add (hi shape, add_hi breakdown) | BinaryAddHi 30/2 → 15, BinaryAddHiLarge 51/4 → 13, BinaryAddHiHuge 93/8 → 12, BinaryAdd* 21–26, Binary* 55–60 | 15 | **12** | 60 |
| `BINARY_E_COST` | sll, srl, sra, *_w, signextend_*, rev8, pack*, rol*, ror*, clz*, ctz*, cpop*, orc_b, bclr, bext, binv, bset, clmul*, xperm*, czero_* | BinaryExtension 58/1 → 58, BinaryExtensionLarge 110/2 → 55 | 56 | **55** | 58 |
| `SLL_U_W_COST` | sll_u_w | = `BINARY_E_COST` | 56 | **55** | 58 |
| `ADD_U_W_COST` | add_u_w | MAIN + BINARY + BINARY_ADD | 153 | **138** | 182 |
| `SH_ADD_COST` | decomposed sh*add | MAIN + BINARY_E + BINARY_ADD | 149 | **138** | 180 |
| `SH_ADD_U_W_COST` | sh1add_u_w, sh2add_u_w, sh3add_u_w | MAIN + BINARY + BINARY | 188 | **172** | 182 |
| `ARITHAM32_COST` / `ARITHA32_COST` | mul*, div*, rem* | Arith 97/1 → 97 | 97 | **97** | 97 |

## Precompiles

| Cost | Opcodes | Instances (Total × rows per op) | Previous | Current | Max |
|---|---|---|---:|---:|---:|
| `KECCAK_COST` | keccak | Keccakf 615 × 135 rows / 2 ops per slot | 41,512 | **41,513** | 41,513 |
| `SHA256_COST` | sha256 | Sha256f 122 × 72 | 8,784 | **8,784** | 8,784 |
| `POSEIDON_COST` | poseidon1, poseidon2 | Poseidon 392 × 14 | 5,488 | **5,488** | 5,488 |
| `POSEIDON2_KOALABEAR_COST` | poseidon2_koalabear | Poseidon2KoalaBear 499 × 30 | 14,970 | **14,970** | 14,970 |
| `BLAKE2B_COST` (per round) | blake2b | Blake2br 233 × 8 | 1,896 | **1,864** | 1,864 |
| `BLAKE2S_COST` | blake2s | Blake2s 115 × 80 | 9,200 | **9,200** | 9,200 |
| `BLAKE3_COST` | blake3 | Blake3f 221 × 56 / 2 lanes | 6,188 | **6,188** | 6,188 |
| `ARITH_256_COST` (new) | arith256, arith256_mod | Arith256X 54 × 16 → 864, ArithEq 90 × 16 → 1,440 | 1,440 | **864** | 1,440 |
| `ARITH_SECP256K1_COST` (new) | secp256k1_add, secp256k1_dbl | ArithSecp256K1 72 × 16 → 1,152, ArithEq → 1,440 | 1,440 | **1,152** | 1,440 |
| `ARITH_BN254_COST` (new) | bn254_curve_add/dbl, bn254_complex_add/sub/mul | ArithBn254 78 × 16 → 1,248, ArithEq → 1,440 | 1,440 | **1,248** | 1,440 |
| `ARITH_EQ_COST` | secp256r1_add, secp256r1_dbl | ArithEq 90 × 16 (the only one proving it) | 1,440 | **1,440** | 1,440 |
| `ARITH_EQ_384_COST` | arith384_mod, bls12_381_* | ArithEq384 80 × 24 | 1,920 | **1,920** | 1,920 |
| `TWISTED_EDWARDS_COST` | babyjubjub_add, ed25519_add | TwistedEdwards 109 × 16 | 1,744 | **1,744** | 1,744 |
| `ADD256_COST` | add256 | Add256 104 × 1 | 104 | **104** | 104 |
| `JUMP_DEST_COST` (per 64-bit source word) | jump_dest | JumpDest 66 / 2 ops x row → 33 | 252 | **33** | 33 |

## DMA (cost per row; the aligned loop takes `div_ceil(words / DIVISOR)` rows)

| Cost | Instances | Previous | Current | Max |
|---|---|---:|---:|---:|
| `DMA_COST` (memcmp, memset) | Dma 63 (single instance, all ops) | 61 | **63** | 63 |
| `DMA_MEMCPY_COST` | Dma 63 | 46 | **63** | 63 |
| `DMA_INPUTCPY_COST` | Dma 63 | 40 | **63** | 63 |
| `DMA_PRE_POST_COST` (memcmp, memset) | DmaPrePost 104 (single instance) | 104 | **104** | 104 |
| `DMA_PRE_POST_MEMCPY_COST` | DmaPrePost 104 | 91 | **104** | 104 |
| `DMA_PRE_POST_INPUTCPY_COST` | DmaPrePost 104 | 59 | **104** | 104 |
| `DMA_64_ALIGNED_MEMCPY_COST` / DIVISOR | Dma64AlignedMemCpy 67/8 (8.4/word), Mem/MemLarge 50/4 (12.5), Dma64Aligned/Large 77/4 (19.3) | 67 / 8 | **67 / 8** | 77 / 4 |
| `DMA_64_ALIGNED_MEMSET_COST` / DIVISOR | Dma64AlignedMemSet 35/8 (4.4/word), Mem/MemLarge 50/4, Dma64Aligned/Large 77/4 | 35 / 8 | **35 / 8** | 77 / 4 |
| `DMA_64_ALIGNED_MEMCMP_COST` / DIVISOR | Dma64AlignedMem/MemLarge 50/4, Dma64Aligned/Large 77/4 | 50 / 4 | **50 / 4** | 77 / 4 |
| `DMA_64_ALIGNED_INPUTCPY_COST` / DIVISOR | Dma64Aligned/Large 77/4 (the only ones proving it) | 58 / 4 | **77 / 4** | 77 / 4 |
| `DMA_UNALIGNED_COST` | DmaUnaligned 42 | 42 | **42** | 42 |

## Memory and base cost (`emulator/src/emu_costs.rs`)

| Cost | Instances | Previous | Current | Max |
|---|---|---:|---:|---:|
| `MEM_READ_COST` | Mem 87/4 → 22 per lane; dual RAM (R+R / W+R) → 11..22 per access | 16 | **11** ¹ | 22 |
| `MEM_WRITE_COST` | Mem 87/4 → 22 per lane | 18 | **18** ¹ | 22 |
| `INPUT_READ_COST` | InputData 29/1 | 29 | **29** | 29 |
| `ROM_READ_COST` / `ROM_INIT_COST` | RomData 22/2 → 11 | 14 | **11** | 11 |
| `MEM_ALIGN_COST` | MemAlign 50, MemAlignLarge 50 | 53 | **50** | 50 |
| `MEM_ALIGN_READ_UNALIGNED_1_COST` | 2 × MEM_ALIGN | 106 | **100** | 100 |
| `MEM_ALIGN_READ_UNALIGNED_2_COST` | 3 × MEM_ALIGN | 159 | **150** | 150 |
| `MEM_ALIGN_WRITE_UNALIGNED_1_COST` | 3 × MEM_ALIGN | 159 | **150** | 150 |
| `MEM_ALIGN_WRITE_UNALIGNED_2_COST` | 5 × MEM_ALIGN | 265 | **250** | 250 |
| `MEM_ALIGN_READ_BYTE_COST` | MemAlignReadByte 25, MemAlignReadByteLarge 25, MemAlignByte 34 | 25 | **25** | 34 |
| `MEM_ALIGN_WRITE_BYTE_COST` | MemAlignWriteByte 32, MemAlignByte 34 | 32 | **32** | 34 |
| `ROM_COST` | Rom 16 × 2²² | 16 << 22 | **16 << 22** | 16 << 22 |
| `TABLES_COST` | VirtualTableZisk1 65 + VirtualTableZisk0 39, × 2²¹ | 105 << 21 | **104 << 21** | 104 << 21 |

¹ Dual RAM: a lane proves R+R or W+R, so an access costs between 11 (full dual lane) and 22 (alone
in its lane). Since not all dual lanes are full, each access is charged a conservative factor of the
dual operation cost (22): read ⌈22 × 0.5⌉ = 11, write ⌈22 × 0.8⌉ = 18.

## Notes

- `sh3add` stays at `BINARY_COST` (general case). Depending on the shape of its operands it can also
  be proven by BinaryAdd (21) or BinaryAddHi (12), but the emulator only does this breakdown for
  `add` (`add_hi0` / `add_hif`).
- Dma, DmaPrePost and the Dma64Aligned inputcpy had costs of specialized instances that do not exist
  in `pil/zisk.pil`; they now pay the full row of the only instance that proves them.
- Keccak: 135 rows per slot holding 2 Keccak-f → 41,512.5, rounded up.
- The precompile costs above are only the area of their airs. The memory accesses of a precompile
  are charged on top, in the MEMORY bucket, by its `ops_*` stats function (each aligned word read or
  written at the RAM/ROM/input cost). They match the memory inputs of each precompile, both the
  direct `a`/`b` calls and the ones that still use a parameter struct (arith256, arith256_mod,
  arith384_mod, add256). Keccak now uses `ops_keccak` (25 reads + 25 writes); before it charged no
  memory.
