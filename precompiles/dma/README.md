# Accelerated Memory Operations

ZisK accelerates the four standard C bulk-memory routines — `memcpy`, `memmove`,
`memset` and `memcmp` — with dedicated DMA precompile circuits instead of proving
them as byte-by-byte RISC-V loops. Compilers emit these functions constantly
(struct moves, slices, serialization, string ops), so accelerating them cuts
proving cost across almost every guest program.

Acceleration is transparent: guests just call the standard functions. No source
changes and no crate patches are required.

## Covered functions

| Function  | Semantics                                              |
|-----------|-------------------------------------------------------|
| `memcpy`  | Copy `n` bytes from src to dst.                       |
| `memmove` | Copy `n` bytes, correct even when regions overlap.   |
| `memset`  | Fill `n` bytes with a byte value.                    |
| `memcmp`  | Unsigned byte comparison; sign of first differing byte. |

All four match libc semantics for every alignment and for zero-length calls, and
make no assumption about operand alignment. `memmove` handles overlapping regions
in both directions.

## How it works

1. **Marker stubs.** ziskos exports strong assembly definitions of the four
   symbols ([`ziskos/entrypoint/src/dma/`](https://github.com/0xPolygonHermez/zisk/tree/main/ziskos/entrypoint/src/dma)).
   Each writes its arguments to a custom CSR and executes a dummy `add` carrying
   the remaining parameters:

   | Function  | CSR      |
   |-----------|----------|
   | `memcpy` / `memmove` | `0x813` |
   | `memcmp`  | `0x814`  |
   | `memset`  | `0x816`  |

2. **Transpiler fusion.** When translating RISC-V to ZisK, the CSR-write + `add`
   pair is fused into a single DMA opcode (`dma_memcpy` `0xd0`, `dma_memcmp`
   `0xd1`, `dma_xmemset` `0xd9`, and their extended variants).

3. **Proving.** DMA precompile circuits
   ([`precompiles/dma/`](https://github.com/0xPolygonHermez/zisk/tree/main/precompiles/dma))
   prove the bulk operation, with sub-machines for the aligned 64-bit fast path,
   the unaligned path, and partial head/tail words — so arbitrary alignments are
   handled without falling back to byte loops.

   Three fused airs let the planner spend fewer instances: `DmaWithPrePost`
   ([`pil/dma_with_pre_post.pil`](pil/dma_with_pre_post.pil)) is the controller
   with its partial writes in one air, `DmaLoop` ([`pil/dma_loop.pil`](pil/dma_loop.pil))
   is the aligned and the unaligned loop in one air, and `DmaCompact`
   ([`pil/dma_compact.pil`](pil/dma_compact.pil)) inlines the two side by side on
   the same rows (pil2-compiler `inline`), so an execution that uses the DMA
   lightly pays a single instance. The strategy offers them to the shared
   selection criterion (fewest instances, least prover memory); for a run,
   `ZISK_DMA_COMPACT=never`, `ZISK_DMA_LOOP=never` or `ZISK_DMA_WITH_PRE_POST=never`
   take an air out of the selection.

## Linking guarantee

The `memcpy`/`memmove`/`memset`/`memcmp` symbols are **strong** definitions in
ziskos, the mandatory guest runtime, so they override `compiler_builtins`' weak
byte-loop fallbacks independently of link order. The guest linker script also
lists them in `EXTERN(...)`, forcing the linker to pull the accelerated
definitions into every guest even under archive (rlib) linking. ZisK therefore
does not rely on link order to make acceleration take effect.

> Note for `libziskos.a` (staticlib) consumers: when linking ziskos into a host
> C/Rust application that already provides its own libc `mem*`, the host controls
> final linking. Use `--whole-archive` (or an equivalent forced-inclusion flag)
> if you want ziskos' accelerated definitions to win there.
