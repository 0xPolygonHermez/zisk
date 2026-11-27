# Plan: MXFP4 and NVFP4 weights with BF16 activations in `dot_bf16`

Status: implemented for both formats, MXFP4 (`flags` = 1) and NVFP4 (`flags` = 2), emulator only, as described below. It extends the `dot_bf16` precompile described in [`README.md`](README.md). The results on the llama2 example, and a comparison of BF16, MXFP4 and NVFP4, are in the [llama2 README](../../examples/llama2/README.md#bf16-mxfp4-and-nvfp4-compared).

## Goal

Run models whose weights are published in MXFP4, such as gpt-oss, the way they usually run: 4-bit weights with BF16 activations ("W4A16"). The dot products keep `dot_bf16`'s semantics: the exact sum, rounded once to `f32`.

`dot_f8` can already multiply E2M1 values, but only by FP4, FP8 or int8 activations, and it leaves the block scales to the guest. This variant takes BF16 activations and applies the MXFP4 scales itself, so a matrix row is one call with no loop in the guest.

## The MXFP4 format (OCP Microscaling)

- **Values:** E2M1, 4 bits each, two per byte, low nibble first: ±{0, 0.5, 1, 1.5, 2, 3, 4, 6}. This is `dot_f8`'s format 3, with the same decoding table.
- **Scales:** one E8M0 byte per block of 32 values: an unsigned exponent with bias 127, value 2^(s − 127), from 2^-127 to 2^127. `0xFF` is NaN.
- **A weight's value** is `e2m1(w[i]) · 2^(s[i / 32] − 127)`.

## The NVFP4 format (NVIDIA)

- **Values:** the same E2M1 values.
- **Scales:** one E4M3 (OCP FP8) byte per block of 16 values: up to 448, with 3 mantissa bits, in units of 2^-9. `0x7F` and `0xFF` are NaN.
- **A per-tensor `f32` scale,** which brings the block scales into E4M3's range. It is not part of the precompile: the guest multiplies each result by it, so a result is rounded twice (once by the precompile, once by that multiplication), as on NVIDIA's hardware, which applies it after the accumulation.
- **A weight's value** is `e2m1(w[i]) · e4m3(s[i / 16])`, times the tensor scale.

## Semantics

`c` = the `f32` bits of the exact sum `Σ e2m1(x[i]) · scale(i) · bf16(y[i])`, rounded once to `f32`, to nearest, ties to even, with `scale(i)` = `2^(s[i / 32] − 127)` for MXFP4 and `e4m3(s[i / 16])` for NVFP4. The rules are `dot_bf16`'s:
- **Blocks and the total:** each `block` result is its own exact sum rounded once, and the total is rounded from the exact total.
- **Zero:** a zero sum is +0.
- **Specials:** an infinite or NaN BF16 value, or a NaN scale (`0xFF` for MXFP4; `0x7F` or `0xFF` for NVFP4), gives the canonical NaN `0x7FC00000`. A negative E4M3 scale is accepted and flips the sign.

**The accumulator doesn't change.**
- **Each product is exact:** an E2M1 significand (2 bits) times a BF16 significand (8 bits) is at most 10 bits, times a power of two.
- **The smallest product** is 2^-1 · 2^-127 · 2^-133 = 2^-261, above the accumulator's lowest bit of 2^-266.
- **The largest** is below 6 · 2^127 · 2^128 < 2^258. With 2^20 products the sum stays below 2^278, far from the sign bit at 2^575.

For NVFP4 the weight's significand is an E2M1 one times an E4M3 one (at most 12 · 229,376 in units of 2^-10), so a product's significand is below 2^30, and its value between 2^-143 and 6 · 448 · 2^128: inside the same range. The accumulator's multiplication was widened to 64 bits for this.

So `Bf16Accumulator`, its rounding and its tests are reused.

## Interface

**`flags` selects the variant.** It is the reserved word of the parameter block:

| `flags` | `x` | `y` | Parameter block |
|---|---|---|---|
| 0 | BF16 | BF16 | 6 words, as today |
| 1 | MXFP4 values (E2M1, packed) | BF16 | 7 words: word 6 is the address of `x`'s scales |
| 2 | NVFP4 values (E2M1, packed) | BF16 | 7 words: word 6 is the address of `x`'s E4M3 scales, one per 16 values |
| other | reserved | | rejected |

**With `flags` = 1 or 2:**
- `n` is a multiple of the block, 32 (MXFP4) or 16 (NVFP4), and at most 2^20.
- `x` holds `n / 2` bytes, 8-byte aligned.
- The scales are `n / 32` or `n / 16` bytes, 8-byte aligned, read as whole words. A last, partial word is read in full and its unused bytes are ignored. They don't need to be zero, so rows whose block count isn't a multiple of 8, such as gpt-oss's 2,880 values (90 blocks), need no padding.
- `y` holds `n` BF16 values, 8-byte aligned.
- `block` is a multiple of the FP4 block dividing `n`, or 0 for one block; `out` works as for `flags` = 0.

**In `ziskos`,** a second parameter struct, `SyscallDotBf16Fp4Params` (7 words, `flags` = 1 or 2), and a function `syscall_dot_bf16_fp4`, both on syscall `0x824`. The existing struct and function are unchanged.

**The minimal trace** keeps `dot_bf16`'s 7-word header (the first 6 words of the parameter block and the result). The scales address is the first word of the data that follows, then the words of `x`, of the scales (rounded up to words) and of `y`.

**The weights go in `x` and the activations in `y`,** the same order as the guest's other calls (`dot(row, activations)`).

## Cost estimate (placeholder)

- **One row per 64-bit word of BF16 activations,** which is 4 multiply-adds, as for `flags` = 0. The FP4 side needs a quarter of a word of values per row, and a scale byte every 8 rows.
- **The same placeholder of about 120 columns per row:**
  - the E2M1 decoding (a 16-entry lookup) is cheaper than a BF16 decoding;
  - the scale adds one exponent addition per product;
  - the accumulator is unchanged.
- **Memory:** the BF16 words, the FP4 words (`n / 16`) and the scale words (`n / 256`, rounded up) are charged as aligned reads.

## Implementation steps

All done, in the same places as `dot_bf16`, with no new opcode or syscall:

1. **`definitions/src/dot_bf16.rs`:**
   - `fp4_weight_decode(flags, code, scale)` into sign, significand and exponent, reusing `DOT_F8_DECODE`'s E2M1 and E4M3 entries;
   - `dot_bf16_fp4_reference(flags, x, scales, y, n, block, out)`, which adds the products to `Bf16Accumulator`;
   - the `DOT_BF16_FLAGS_*` constants.
2. **`core/src/operations/dot_bf16.rs`:**
   - read `flags`; for 1, read the 7th word and check `n`, `block` and the alignments;
   - read the FP4 and scale words, call the reference, and charge the extra reads in the cost model.
3. **`ziskos`:** `SyscallDotBf16Fp4Params` and `syscall_dot_bf16_fp4`, the latter calling the reference off the ZisK target.
4. **The assembly emulator** (`emulator-asm/src/dot_bf16/`):
   - in the C kernel, a second loop that decodes E2M1 through a 16-entry table, and the E4M3 scale through a 256-entry one, and adds the weight's exponent to the product position;
   - the minimal trace gets the 7-word block, the result, then the words of `x`, the scales and `y`;
   - the wrapper's size computation reads `flags`.
5. **Tests,** below.
6. **llama2:**
   - `--weights mxfp4` quantizes to MXFP4 with the OCP recipe: per block of 32, the scale exponent is `floor(log2(max|w|)) − 2`, and the values are rounded to E2M1 to nearest even, saturating at ±6;
   - `--weights nvfp4` quantizes to NVFP4: a tensor scale `max|w| / (6 · 448)` per layer's matrix, then per block of 16 an E4M3 scale `max|w| / 6 / tensor scale`, rounded to nearest even, and the values rounded to E2M1;
   - the activations are BF16;
   - measure accuracy and cost against BF16 and int8, as in the README.
7. **Later:** a real MXFP4 model. gpt-oss-20b (about 12 GB) doesn't fit in the 2 GB input region, so this means a smaller MXFP4 checkpoint, or a model quantized to MXFP4 on the host.

## Basic tests

### Reference (`definitions`)

`n` = 32 unless stated. The values are listed for the first few positions; the rest are 0. All expected results are exact.

| # | Case | Expected `c` | What it checks |
|---|---|---|---|
| 1 | `x` = 1, 2, 3, 4 (E2M1), scale 2^0, `y` = 1 (BF16) | 10.0 | basic decoding |
| 2 | as 1, scale 2^-3 | 1.25 | the scale is applied |
| 3 | `x` = 6, −6, scale 2^127, `y` = 2^100, 2^100 | +0 | exact cancellation at the top of the range |
| 4 | `x` = 0.5, scale 2^-127, `y` = 2^-133 (smallest BF16 subnormal) | +0 | the smallest product (2^-261) underflows to 0 |
| 5 | `x` = 6, scale 2^127, `y` = BF16 max | +∞ | overflow in the final rounding |
| 6 | `n` = 64: `x` = 1 at the start of each block, scales 2^10 and 2^-20, `y` = 1 | 1024.0 (1024 + 2^-20 rounds down) | one scale per block of 32 |
| 7 | as 6 with `block` = 32 | `out` = [1024.0, 2^-20] | block results |
| 8 | a scale of `0xFF` | NaN `0x7FC00000` | NaN scale |
| 9 | `y` with ∞ | NaN | BF16 specials, as for `flags` = 0 |
| 10 | `n` = 64: `x` = 1 at the start of block 0 (scale 2^0) and 0.5 at the start of block 1 (scale 2^-23), `y` = 1: exact sum 1 + 2^-24 | 1.0 | a tie, to even, across blocks |
| 11 | 5,000 random rows per format, `n` up to 2,048, MXFP4 scale exponents in [−10, 10] or any finite E4M3 scale, BF16 exponents in [−8, 8] | equal to an `i128` oracle | random agreement |
| 12 | random rows over the full ranges | the same result reversed by blocks | order independence |
| 13 | every pair of an E2M1 code and an E8M0 or E4M3 scale | `fp4_weight_decode` against `f64`, and NaN for the NaN codes | exhaustive decoding (2 × 16 × 256 pairs) |
| — | NVFP4: fractional E4M3 scales (1.5, 0.375), the largest (448), the smallest subnormal (2^-9), a negative scale | exact | E4M3 scales |

### Core

- **Parameter checks panic** with a clear message:
  - `flags` other than 0 or 1;
  - `n` not a multiple of 32;
  - `block` not a multiple of 32 or not dividing `n`;
  - an unaligned `x`, scales, `y` or `out`.
- **The cost model** charges the three vectors' words.
- **A partial last scale word** (for example 90 blocks) is read in full and its extra bytes are ignored. A test sets them to `0xFF`; the result must not be NaN.

### Guest test program

The `test-artifacts/programs/syscalls/dot_bf16` guest gains these cases through `syscall_dot_bf16_mxfp4`, plus random rows of 2,880 values (90 blocks) checked against the reference in the guest. It must pass on `ziskemu` and on the assembly emulator (fast and minimal trace), with the same step count.

### Cross-check of the C kernel

The random-case generator and checker now cover all three variants, including NaN scales, and fill the bytes after the scales with `0xFF` to check that they are ignored. They are in the repository: [`emulator-asm/tests/dot_bf16/run.sh`](../../emulator-asm/tests/dot_bf16/run.sh).

## Open questions

1. **The order of the operands.** The weights are always in `x` here. Allowing MXFP4 in either operand, or in both (MXFP4 by MXFP4, as Blackwell does), would mean more `flags` values. I'd leave that until a model needs it.
2. **NVFP4** is implemented as `flags` = 2. Its per-tensor scale stays in the guest, which rounds a result a second time; including it in the precompile would need a wider accumulator.
3. **Rounding the activations.** They are rounded to BF16 before each matrix multiplication, as in the BF16 mode. That matches how W4A16 models run.
4. **The state machine** is still the main unknown, as for `dot_bf16` itself. The FP4 path adds a lookup and an exponent addition per product, but no new accumulator.
