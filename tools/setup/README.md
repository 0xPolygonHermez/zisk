# Zisk setup scripts

`build-setup.sh` builds the Zisk proving key locally (`compile-pil` + `setup`)
via the `cargo-zisk` pipeline. An optional `--cache-dir` reuses a previously
built `provingKey/` keyed by the input hash. It never touches any bucket /
network.

To **package** the result into tarballs (and optionally upload), use
[`tools/test-env/package_setup.sh`](../test-env/package_setup.sh).

## One-time setup

### Tools you need on PATH

| Tool | Why |
|---|---|
| Rust toolchain (stable) | builds `cargo-zisk` and the proofman-setup binary |
| Node.js + npm | `compile-pil` shells out to the JS pil2-compiler (installed on demand) |
| `circom` | required by `setup --recursive` (recursive circuit compilation) |
| Standard Unix utils | `tar`, `find`, `awk`, `sed`, `sort`, sha256 (`sha256sum` on Linux, `shasum` on macOS — auto-detected) |

### Platform support

`build-setup.sh` is Linux/x86_64 only because the underlying proving-key pipeline
depends on `nasm` and the C++ STARK lib's Linux toolchain (see the apt list
below).

### System packages (Ubuntu / Debian)

These are the build deps for pil2-proofman and the C++ STARK lib it builds:

```bash
sudo apt-get install -y protobuf-compiler build-essential cmake nasm \
    libbenchmark-dev libomp-dev libgmp-dev libsodium-dev nlohmann-json3-dev \
    openmpi-bin openmpi-common libopenmpi-dev
```

### Repo-side setup

`build-setup.sh` resolves the pil2-proofman checkout from the `proofman` git rev
in `Cargo.lock` — cargo's own checkout under `~/.cargo/git/checkouts/` is reused,
and the script runs `npm install` there on first use. Set
`PROOFMAN_DIR=/path/to/local/pil2-proofman` to override (required when proofman
is a local path dep, e.g. dev work on an unpushed branch).

```bash
# Install circom (one-time, from iden3/circom)
git clone https://github.com/iden3/circom /tmp/circom && \
  (cd /tmp/circom && cargo install --path circom)
```

## Common workflows

`build-setup.sh` modes are mutually exclusive.

### Build a proving key

```bash
./tools/setup/build-setup.sh
```

Runs frops generators, `compile-pil`, then `setup --recursive`. Result lands in
`build/provingKey/`. Nothing is uploaded.

### Reuse a prior build via the local cache

```bash
./tools/setup/build-setup.sh --cache-dir /path/to/cache
```

On a cache hit (`<cache-dir>/<platform>/<input-hash>/provingKey/` exists), that
`provingKey/` is copied into `build/` and `compile-pil` + `setup` are skipped. On
a miss, the fresh build is copied back into the cache. The key is
`PLATFORM/<input-hash>` (the aggregation mode is folded in, so a recursive and a
`--no-aggregation` build never collide). This is a plain filesystem cache — no
bucket or network access. Used by `tools/test-env/build_setup.sh` to make repeat
CI runs cheap.

### Build without aggregation (debug / fast iteration)

```bash
./tools/setup/build-setup.sh --no-aggregation
```

Runs `setup` without `-r`. Output stays in `build/provingKey/`.

### Snark only

```bash
./tools/setup/build-setup.sh --snark                                  # produces <build-dir>/provingKeySnark/
./tools/setup/build-setup.sh --snark --build-dir /path/to/build       # use an existing provingKey at this path
```

Requires an existing `<build-dir>/provingKey/` (run `build-setup.sh` without
`--snark` first). Errors out if it's missing.

`--build-dir <path>` defaults to `build/` and applies to every mode that touches a
build directory (default, `--no-aggregation`, `--snark`, `--compressed-final`).

### Re-run only vadcop_final_compressed

```bash
./tools/setup/build-setup.sh --compressed-final
```

Skips compressor / recursive1 / recursive2 / vadcop_final and re-runs only
`vadcop_final_compressed` on top of an existing
`<build-dir>/provingKey/<name>/vadcop_final/`. Useful when iterating on the
compressed_final stage; you don't pay for the full recursive pipeline.

### Reuse an existing pilout

```bash
./tools/setup/build-setup.sh --skip-compile-pil
```

Skips `compile-pil` and reuses `pil/zisk.pilout`. Faster local iteration when only
setup-side code changed. With `--cache-dir`, the cache is NOT populated (the
reused pilout may not match the computed input hash).

### Parallelism

```bash
./tools/setup/build-setup.sh --recursive-jobs 4 --setup-jobs 8
```

- `--recursive-jobs N` — concurrent recursive1 air pipelines (circom + pil2com). Heavy: each job can use several GB. Pick N based on `floor(available_RAM / per_air_peak)`.
- `--setup-jobs N` — concurrent non-recursive AIR setups (pil_info + I/O). Cheaper.
- Both default to 1 (serial). `RECURSIVE_JOBS` / `SETUP_JOBS` env vars work too; the CLI flag wins if both are set.

### Per-AIR statistics

```bash
./tools/setup/build-setup.sh --stats
```

Generates frops fixed data, compiles `pil/zisk.pilout`, runs `proofman-setup stats`. Output to `tmp/stats.txt`.

### Package the result

```bash
(cd tools/test-env && ./package_setup.sh)
```

[`package_setup.sh`](../test-env/package_setup.sh) tars `provingKey/`, the verify
key, and — when present — `circom/` and `provingKeySnark/` into `${OUTPUT_DIR}`
(with `.md5` sidecars). It is local-only by default; set `PACKAGE_SETUP_UPLOAD=1`
to also upload to `gs://zisk-setup` (requires `gcloud` auth).

## Direct CLI access

The orchestrator wraps these — call them directly if you need finer control:

```bash
# Compile .pil → .pilout (wraps the JS pil2-compiler; npm install required)
# build-setup.sh passes the std-pil include as <proofman-checkout>/pil2-components/lib/std/pil
cargo zisk proofman-setup compile-pil \
  --pil pil/zisk.pil \
  -I pil,<proofman-checkout>/pil2-components/lib/std/pil,state-machines,precompiles \
  -o pil/zisk.pilout \
  --fixed-dir tmp/fixed --fixed-to-file

# Generate the proving key from a pilout
cargo zisk proofman-setup setup \
  --airout pil/zisk.pilout \
  --build-dir build \
  --fixed-dir tmp/fixed \
  --stark-structs state-machines/starkstructs.json \
  --recursive

# Final SNARK setup (after the previous one succeeds)
cargo zisk proofman-setup setup-snark --build-dir build \
  --publics-info state-machines/publics.json \
  --powers-of-tau ../powersOfTau28_hez_final_27.ptau

# Per-AIR statistics
cargo zisk proofman-setup stats --airout pil/zisk.pilout -o /tmp/stats.txt

# Rebuild every witness library in an existing provingKey
cargo zisk proofman-setup rebuild-witness-libs --proving-key build/provingKey

# Help
cargo zisk proofman-setup --help
```

## Cache key

`build-setup.sh` computes an input hash (in `lib/setup-common.sh`) used as the
`--cache-dir` key. It is a sha256 over (in this order):

- All `*.pil` under `pil/`, `state-machines/`, `precompiles/` (paths sorted with `LC_ALL=C` for cross-host stability)
- All `*.pil` under `${PROOFMAN_DIR}/pil2-components/lib/std/pil`
- `state-machines/starkstructs.json`
- `state-machines/{arith,binary}/src/*_frops_fixed.bin` (regenerated automatically before hashing)
- `pil2-compiler` dep ref from `${PROOFMAN_DIR}/package.json`
- `pil2-stark-setup` `source` (e.g. `git+https://github.com/.../pil2-proofman.git?branch=X#<sha>`) from `Cargo.lock`

The script aborts if `pil2-stark-setup` has no git `source` in `Cargo.lock` (e.g.
resolved as a local path dep) — a path would tie the cache key to one host.
Bumping any of these — including the proofman or pil2-compiler git ref —
invalidates the cache and forces a rebuild on the next `build-setup.sh` run.
Metadata is parsed with `awk`/`sed` over `Cargo.lock` + `package.json`; no `jq`.
