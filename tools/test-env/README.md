# ZisK Test Environment

## Build docker image
To build the `zisk-test-env` Docker image, execute the following command:

```bash
./build_docker.sh
```

## Run docker container
To run the Docker container, execute the following command:

```bash
./run_docker.sh
```

This will run the Docker container and open the ZisK test menu inside the container. If the container already exists, you will be asked whether you want to connect to it or recreate it.

>[!CAUTION]
>
>If you choose to recreate the container, all existing content inside it will be lost.

> [!TIP]
>
>The Docker container includes a `${HOME}/output` directory, which is mapped to the `./output` folder on the host.
>You can use this folder to copy any files you want to make available outside of the container.

## ZisK Test Menu Options

1. **Edit environment variables**
   Opens the `.env` file with the `nano` editor, allowing you to modify environment variable values.
   These variables let you specify the repository branches to use, the setup version to generate or install, and the parameters to use when proving in distributed mode.

2. **Build ZisK from source**
   Builds ZisK from the `zisk` repository source (the branch in `ZISK_BRANCH`, unless a local `ZISK_REPO_DIR` is used). pil2-proofman is consumed as the git dependency pinned in ZisK's `Cargo.toml` / `Cargo.lock` — it is no longer cloned or branch-overridden.
   After building, it installs the CLI tools and necessary files to the `$HOME/.zisk` folder and adds that folder to the `$PATH` environment variable.

3. **Build setup from source**
   Builds the setup files (proving key) by delegating to `tools/setup/build-setup.sh`, which runs the `cargo-zisk` proving-key pipeline (`compile-pil` + `setup`). It no longer clones `pil2-proofman-js` / `pil2-compiler` or shells into node — `pil2-compiler` is pulled via npm at the version pinned in `pil2-proofman`'s `package.json`, and the proofman checkout is whatever `Cargo.toml` resolves to (set up by option 2). With `USE_CACHE_SETUP=1` a local artifact cache under `${HOME}/output` is reused/populated, keyed by the input hash.
   After building, it installs the proving key to the `$HOME/.zisk` folder and generates the constant files using the `cargo-zisk-dev check-setup` command.

4. **Build zec-reth ELF**
   Clones the `zisk-eth-client` repository (branch specified by `ZISK_ETH_CLIENT_BRANCH`) and patches its `bin/guests/stateless-validator-reth/Cargo.toml` so that the `ziskos` dependency points to the local ZisK repository resolved from `ZISK_REPO_DIR` (or `${HOME}/workspace/zisk` if unset). It then builds the guest with `cargo-zisk build --release` and verifies that `target/elf/riscv64ima-zisk-zkvm-elf/release/zec-reth` was produced.
   The resulting ELF is consumed by options **8. Test Ethereum Block** and **9. Test EthProofs**, so this option must be run before either of them.

5. **Package setup outcome**
   Packages the setup artifacts (`.tar.gz` + `.md5`) from the files generated in option 3: the proving key and verify key always, plus the circom circuits (`zisk-circuits`) and snark proving key (`zisk-provingkey-plonk`) when present in `build/`.
   The artifacts are stored in the `${HOME}/output` directory inside the container, which is mapped to the `./output` folder on the host, making them available externally. Set `PACKAGE_SETUP_UPLOAD=1` to also upload them to `gs://zisk-setup` (requires `gcloud` auth); by default packaging is local-only.

6. **Install ZisK from binaries**
   Installs ZisK from binaries using the latest official release via `ziskup`.

7. **Test sha_hasher**
   Creates, builds, and emulates the `sha_hasher` program, then generates and verifies the proof.
   It also performs constraints verification.

8. **Test Ethereum Block**
   Tests Ethereum block proof generation using the `zec-reth` ELF and the input files cloned by option **4. Build zec-reth ELF** (which must be run beforehand).
   First, it proves the input files specified in the `BLOCK_INPUTS_SINGLE` environment variable using cargo-zisk with one single process (no mpi). Second, it proves the input files specified in the `BLOCK_INPUTS_MPI` environment variable using cargo-zisk and mpi with the number of processes and threads specified in `MPI_PROCESSES` and `MPI_THREADS` environment variables.

9. **Test EthProofs**
   Clones the `zisk-ethproofs` repository, builds it, and deploys the `zisk-coordinator` and `zisk-worker` services. Requires the `zec-reth` ELF and inputs produced by option **4. Build zec-reth ELF** (which must be run beforehand).
   Then runs the `ethproofs-client` binary against the deployed coordinator using the input files specified in `BLOCK_INPUTS_ETHPROOFS` (or `BLOCK_INPUTS_ETHPROOFS_HINTS` when `ENABLE_HINTS=1`).
   The distributed services are automatically uninstalled when the test finishes.

10. **Test ELF diagnostic**
    Runs the diagnostic ELF built from the zisk repo's `test-artifacts` crate (`test-artifacts/programs/target/elf/riscv64ima-zisk-zkvm-elf/release/diagnostic`) through the full proving pipeline using `test_elf` (verify-constraints, prove, verify) with no input file.

11. **Install setup from public packages**
    Downloads and installs the proving key files from the public packages corresponding to the `ZISK_SETUP_FILE` environment variable (falling back to a name derived from the installed `cargo-zisk-dev` version when unset).
    After installation, it generates the constant files using the `cargo-zisk-dev check-setup` command.

12. **Install setup from local packages**
    Installs the proving key files using the setup packages generated by option `5. Package setup outcome`, which must be located in the `${HOME}/output` directory.
    After installation, it generates the constant files using the `cargo-zisk-dev check-setup` command.

13. **Shell**
    Opens a command line shell inside the container.
    When you exit the shell, you will return to the ZisK Test Menu.

14. **Exit**
    Exits the Release Kit container and returns to the host shell.

## Publishing a setup to the cloud (CI)

Setups are published to `gs://zisk-setup` by the manual **Upload Setup to Cloud**
workflow (`.github/workflows/upload-setup.yml`), triggered from the Actions tab →
"Run workflow". It publishes only when you explicitly decide a setup is final.

Inputs:
- **version** — published name/version. Blank ⇒ defaults to
  `gha_package_setup_version` in `Cargo.toml`. Override for provisional setups.
- **include_snark** — build + publish the SNARK artifacts. Only honored on `main`.
- **skip_macos** — publish a Linux-only setup (escape hatch if the macOS runner
  is broken).
- **force** — bypass the input-hash newness check.
- **ptau_url** — powers-of-tau URL for the SNARK setup (downloaded once and
  cached on the runner). Defaults to the `_24` Hermez ptau.

The workflow runs four jobs:
1. `build-and-stage` (Linux): builds the proving key **fresh** into a persistent
   host path (so the per-circuit `.cpp` files are persisted), builds the SNARK
   part on `main` (with `include_snark`), runs the newness gate, and uploads a
   small `.cpp`/`.so`/`.globalInfo.json` slice (of `provingKey/` and, on `main`,
   `provingKeySnark/`). Publishing is skipped if the build-input hash already
   matches `gs://zisk-setup/zisk-provingkey-<version>.hash` (unless `force`).
2. `macos-dylibs` (macOS): rebuilds the `.dylib`s from that slice via
   `cargo-zisk proofman-setup rebuild-witness-libs` — including the snark
   `recursivef`/`final` libs (`--proving-key-snark`) when the snark slice is
   present (unless `skip_macos`).
3. `merge-and-upload` (same Linux runner, reusing the staged tree in place):
   merges the dylibs, packs the tarballs, uploads them, and finally writes the
   `.hash` sidecar.
4. `cleanup-stage` (always): removes the staged tree to reclaim disk.

The large proving key never leaves the Linux runner — only the small slice and
the dylibs cross machines.

**Local devs:** `./fetch_setup.sh` downloads + installs the latest published
setup (and records a `.setup-hash` marker). Check whether your installed setup
is current with `./fetch_setup.sh --check installed`.

**Prerequisite (repo owner):** a GCS write credential must be configured as repo
secrets — either Workload Identity Federation (`GCP_WIF_PROVIDER`,
`GCP_SERVICE_ACCOUNT`) or a service-account key (`GCP_SA_KEY`, which requires
editing the workflow's `auth` step to use `credentials_json`). The principal
needs object read + write on `gs://zisk-setup`.
