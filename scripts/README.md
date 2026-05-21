# Zisk setup scripts

Dev-side helpers for generating, packaging, and publishing the Zisk proving key.

## One-time setup

### Tools you need on PATH

| Tool | Why |
|---|---|
| Rust toolchain (stable) | builds `cargo-zisk` and the proofman-setup binary |
| Node.js + npm | `compile-pil` shells out to the JS pil2-compiler |
| `circom` | required by `setup --recursive` (recursive circuit compilation) |
| `jq` | `build-setup.sh` parses Cargo / package.json metadata |
| `curl` | `build-setup.sh` / `fetch-setup.sh` download from the public bucket — no auth needed |
| `gcloud` SDK (`gcloud storage`) | only for `package-proving-key.sh` uploads — auth required |
| Standard Unix utils | `tar`, `sha256sum`, `md5sum`, `find`, `awk` |

### System packages (Ubuntu / Debian)

These are the build deps for pil2-proofman and the C++ STARK lib it builds:

```bash
sudo apt-get install -y protobuf-compiler build-essential cmake nasm \
    libbenchmark-dev libomp-dev libgmp-dev libsodium-dev nlohmann-json3-dev \
    openmpi-bin openmpi-common libopenmpi-dev
```

### Repo-side setup

`build-setup.sh` resolves the pil2-proofman checkout from the `proofman` git dep
in `Cargo.toml` — cargo's own checkout under `~/.cargo/git/checkouts/` is reused,
and the script runs `npm install` there on first use. Set
`PROOFMAN_DIR=/path/to/local/pil2-proofman` to override (e.g. dev work on an
unpushed branch).

```bash
# Install circom (one-time, from iden3/circom)
git clone https://github.com/iden3/circom /tmp/circom && \
  (cd /tmp/circom && cargo install --path circom)
```

### GCS auth (publishing only)

`gs://zisk-setup` is public-read, so `build-setup.sh` cache lookups and downloads go over anonymous `curl` — **no auth**. Auth is only required when you intend to publish via `package-proving-key.sh` (which shells out to `gcloud storage`):

```bash
gcloud auth login
# or: export GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json
```

## Publishing locally — the short version

```bash
# 1. one-time prereqs above
# 2. build (cache miss → runs full setup, drops build/.input-hash). No auth needed.
./scripts/build-setup.sh
# 3. authenticate to GCS (only required for the publish step)
gcloud auth login
# 4. publish (uploads tarballs + the hash sidecar)
./scripts/package-proving-key.sh --build-dir build
```

`build-setup.sh` regenerates frops fixed data → computes the input hash → checks the bucket → either downloads (cache hit, nothing else to do) or runs `compile-pil` + `setup --recursive` and writes `build/.input-hash`. `package-proving-key.sh` then packages `provingKey/` + `circom/`, uploads them, and — if it sees `build/.input-hash` — uploads the `.input-hash` sidecar so the next build-setup run hits cache.

## Common workflows

Two entry points:
- **`build-setup.sh`** — cache lookup; on miss, builds locally. Use when you may need to produce the artifact yourself.
- **`fetch-setup.sh`** — cache lookup only; never builds. Use when you only want the published key (CI consumers, downstream users).

`build-setup.sh` modes are mutually exclusive.

### Release vs pre-release namespace

Both scripts and `package-proving-key.sh` default to the `pre-<VERSION>` namespace (e.g. `zisk-provingkey-pre-1.0.0-beta.*`), so day-to-day publishing doesn't overwrite the stable artifact. Pass `--release` to all three to operate on the bare `<VERSION>` namespace — these must stay in sync (an upload with `--release` and a lookup without it will silently miss the cache).

### Get a working proving key (cache or build)

```bash
./scripts/build-setup.sh
```

Computes an input hash, checks `gs://zisk-setup/zisk-provingkey-pre-<VERSION>.input-hash`:
- **Cache hit**: downloads `zisk-provingkey-pre-<VERSION>.tar.gz`, extracts to `$HOME/.zisk/provingKey/`. Done.
- **Cache miss**: runs frops generators, `compile-pil`, then `setup --recursive`. Result lands in `build/provingKey/`. Nothing uploaded.

### Fetch the published proving key (cache only, no build)

```bash
./scripts/fetch-setup.sh                                    # extracts to build/provingKey/
./scripts/fetch-setup.sh --build-dir /path/to/out           # extract elsewhere
./scripts/fetch-setup.sh --release                          # release namespace (bare <VERSION>)
./scripts/fetch-setup.sh --force                            # skip the hash compare, take whatever's published
./scripts/fetch-setup.sh --skip-compile-pil                 # reuse existing *_fixed.bin for the hash
```

Same input-hash compare as `build-setup.sh`. Exits 0 on cache hit (tarball extracted into `<build-dir>/provingKey/`, replacing any existing one) and exits 2 on miss without attempting a build. Use `--force` when you don't want to pay for `cargo run` + frops generation just to test if the bucket has something publishable.

### Build and publish a new proving key

```bash
./scripts/build-setup.sh                                     # build (cache miss → drops build/.input-hash)
./scripts/package-proving-key.sh --build-dir build           # upload tarballs + hash sidecar (pre- namespace)
./scripts/package-proving-key.sh --build-dir build --release # upload under the bare <VERSION>
```

`build-setup.sh` only writes `<build-dir>/.input-hash` when it actually built something from a fresh `compile-pil` run. On cache hit, or with `--skip-compile-pil`, no sidecar is written and `package-proving-key.sh` will skip the sidecar upload (warning printed) so the cache isn't refreshed with a hash that doesn't match the artifacts.

### Build without aggregation (debug / fast iteration)

```bash
./scripts/build-setup.sh --no-aggregation
```

Bypasses the bucket entirely (cached artifacts are always recursive). Runs `setup` without `-r`. Output stays in `build/provingKey/`.

### Snark only

```bash
./scripts/build-setup.sh --snark                                  # produces <build-dir>/provingKeySnark/
./scripts/build-setup.sh --snark --build-dir /path/to/build       # use an existing provingKey at this path
./scripts/package-proving-key.sh --build-dir build --snark        # upload zisk-provingkey-plonk-<VERSION>.tar.gz
```

- If `<build-dir>/provingKey/` already exists, uses it as-is.
- Otherwise checks the bucket; cache hit → downloads + extracts to `<build-dir>/`, then runs `setup-snark`.
- Cache miss → errors out (no recursive proving key cached for current inputs). Run without `--snark` first to build it.

`--build-dir <path>` defaults to `build/` and applies to every mode that touches a build directory (default, `--no-aggregation`, `--snark`).

### Re-run only vadcop_final_compressed

```bash
./scripts/build-setup.sh --compressed-final
```

Skips compressor / recursive1 / recursive2 / vadcop_final and re-runs only
`vadcop_final_compressed` on top of an existing `<build-dir>/provingKey/<name>/vadcop_final/`.
Useful when iterating on the compressed_final stage; you don't pay for the full
recursive pipeline. No bucket interaction.

### Reuse an existing pilout

```bash
./scripts/build-setup.sh --skip-compile-pil
```

Skips `compile-pil` and reuses `pil/zisk.pilout`. Faster local iteration when only setup-side code changed. Does NOT write `<build-dir>/.input-hash` (the hash would not match the stale pilout), so a follow-up `package-proving-key.sh` run will upload the tarballs but skip the sidecar — i.e. the cache stays as-is.

### Parallelism

```bash
./scripts/build-setup.sh --recursive-jobs 4 --setup-jobs 8
```

- `--recursive-jobs N` — concurrent recursive1 air pipelines (circom + pil2com). Heavy: each job can use several GB. Pick N based on `floor(available_RAM / per_air_peak)`.
- `--setup-jobs N` — concurrent non-recursive AIR setups (pil_info + I/O). Cheaper.
- Both default to 1 (serial). `RECURSIVE_JOBS` / `SETUP_JOBS` env vars work too; the CLI flag wins if both are set.

### Per-AIR statistics

```bash
./scripts/build-setup.sh --stats
```

Generates frops fixed data, compiles `pil/zisk.pilout`, runs `proofman-setup stats`. Output to `tmp/stats.txt`. No bucket interaction.

### Just package what's already in `build/`

```bash
./scripts/package-proving-key.sh --build-dir ./build           # provingKey + circuits
./scripts/package-proving-key.sh --build-dir ./build --snark   # snark output only
./scripts/package-proving-key.sh --build-dir ./build --all     # all three
```

Always uploads to `gs://zisk-setup/`. Tarballs are also kept locally in `./dist/`.

## Direct CLI access

The orchestrator wraps these — call them directly if you need finer control:

```bash
# Compile .pil → .pilout (wraps the JS pil2-compiler; npm install required)
cargo zisk proofman-setup compile-pil \
  --pil pil/zisk.pil \
  -I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines,precompiles \
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
cargo zisk proofman-setup setup-snark --build-dir build

# Per-AIR statistics
cargo zisk proofman-setup stats --airout pil/zisk.pilout -o /tmp/stats.txt

# Rebuild every witness library in an existing provingKey
cargo zisk proofman-setup rebuild-witness-libs --proving-key build/provingKey

# Help
cargo zisk proofman-setup --help
```

## Cache key

Both `build-setup.sh` and `fetch-setup.sh` use the same shared hash, computed by `lib/setup-common.sh` over (in this order):

- All `*.pil` under `pil/`, `state-machines/`, `precompiles/` (paths sorted with `LC_ALL=C` for cross-host stability)
- All `*.pil` under `${PROOFMAN_DIR}/pil2-components/lib/std/pil`
- `state-machines/starkstructs.json`
- `state-machines/{arith,binary}/src/*_frops_fixed.bin` (regenerated automatically before hashing)
- `pil2-compiler` dep ref from `${PROOFMAN_DIR}/package.json`
- `pil2-stark-setup` `.source` (e.g. `git+https://github.com/.../pil2-proofman.git?branch=X#<sha>`) from `cargo metadata`

The script aborts if `pil2-stark-setup` lacks a `.source` (e.g. resolved as a local path dep) — falling back to its manifest path would tie the cache key to one host. Bumping any of these — including the proofman or pil2-compiler git ref — invalidates the cache and forces a rebuild on the next `build-setup.sh` run.

## Bucket layout

Flat in `gs://zisk-setup/`, version in the filename. The default (pre-release) namespace prefixes `<VERSION>` with `pre-`; `--release` uploads use the bare `<VERSION>`:

```
# pre-release (default)
zisk-provingkey-pre-<VERSION>.tar.gz       # provingKey/
zisk-provingkey-pre-<VERSION>.tar.gz.md5
zisk-provingkey-pre-<VERSION>.input-hash   # cache key sidecar
zisk-circuits-pre-<VERSION>.tar.gz         # circom/
zisk-provingkey-plonk-pre-<VERSION>.tar.gz # provingKeySnark/  (only after setup-snark)

# release (--release on every script)
zisk-provingkey-<VERSION>.tar.gz
zisk-provingkey-<VERSION>.tar.gz.md5
zisk-provingkey-<VERSION>.input-hash
zisk-circuits-<VERSION>.tar.gz
zisk-provingkey-plonk-<VERSION>.tar.gz
```

`<VERSION>` = workspace `[workspace.package].version` from `Cargo.toml`.
