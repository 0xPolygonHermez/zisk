# zisklib generator scripts

Several zisklib `.zisk` files are generated, fully or partly, by the Python
scripts here. A file's header names the script that wrote it. Edit the script and
regenerate; a hand edit to a generated file is lost the next time it is
regenerated.

The generators need only Python 3 (no packages) and are run from any directory.

| Script | Writes | How to regenerate |
|---|---|---|
| `ripemd160_gen.py` | `zkvm/ripemd160.zisk` (whole file) | `python3 ripemd160_gen.py` |
| `modexp_gen.py` + `modexp_mont.py` | `zkvm/modexp.zisk`: the direct and Montgomery paths, in front of the general path | see below |
| `secp256k1_glv4_gen.py` | `secp256k1/glv4.zisk` (whole file) | `python3 secp256k1_glv4_gen.py` |
| `pairing/build.sh` + `pairing/*.py` | the optimized `bn254/*` and `bls12_381/*` files, `kernels.zisk`, `zkvm/bn254.zisk`, `zkvm/bls12_381.zisk` | `bash pairing/build.sh` |
| `benchmark/` | nothing: per-call benchmarks and correctness checks of the `zkvm_*` functions | see [benchmark/README.md](benchmark/README.md) |
| `legacy/gen_*.py` | first versions of the `bn254`, `bls12_381` and `bigint` files | not for regenerating (see below) |

## modexp

`modexp_gen.py` edits a file in place. Its input is the general-path source,
which is `zkvm/modexp.zisk` as it was before the rewrite:

```
git show ca6bbb955:ziskasm/zisklib/zkvm/modexp.zisk > modexp.zisk
python3 modexp_gen.py modexp.zisk
mv modexp.zisk ../zkvm/modexp.zisk
```

`modexp_mont.py` holds the Montgomery path (CIOS multiplication and the
dedicated squaring). `modexp_gen.py` imports it, so it is not run on its own.

## bn254 / BLS12-381 (`pairing/`)

`build.sh` takes the pre-optimization sources from git (commit `c2324b455`,
overridable with `BASE=`) and runs them through these steps:

1. `prep.py`: source edits. It adds the batched Miller loop (`mlbatch.py`), the
   unrolled G1 scalar multiplication (`smul.py`) and the unrolled BLS G1
   subgroup multiply (`sgx.py`), and changes the pairing and KZG call sites.
2. `zopt.py`: a peephole pass. It inlines the static-argument Fp and Fp2 leaves,
   the memcpy and compare helpers, and the BE <-> limb conversions.
3. `hints.py`: unrolls the line-coefficient hint reads.
4. `kern.py` with `tc.py`: the tower compiler. It writes `kernels.zisk`, with
   straight-line Fp6 and Fp12 mul, square and sparse mul, plus the cyclotomic
   square. `strip.py` then drops the routines those kernels replace.

`fp2.zisk` is hand-written, and `build.sh` reads it from the working tree.

`bash pairing/build.sh DIR` writes into `DIR` instead of the library, so the
output can be compared with the committed files before replacing them.

## legacy/

These scripts generated the first versions of the `bn254`, `bls12_381` and
`bigint` files. They are kept for reference, because those files still name
them in their headers. Some of their outputs were fixed by hand afterwards:

- `bls12_381/verify.zisk`: canonical infinity-point checks;
- `bigint/common.zisk` and `bigint/modexp.zisk`: the 132-word bit-decomposition
  buffers and their length guards;
- `bls12_381/hash.zisk`: a blank line.

The bn254 and BLS12-381 outputs were also optimized by `pairing/` since. Each
legacy script therefore requires an output directory and never writes into
zisklib: `python3 legacy/gen_fp6.py /tmp/out`.
