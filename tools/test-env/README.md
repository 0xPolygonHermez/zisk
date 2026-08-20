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

4. **Build dylib files (macOS)**
   Rebuilds the macOS witness libraries from the proving key produced by option **3. Build setup from source** (and the snark proving key, when present) and collects the resulting `*.dylib` into `build/dylib`, preserving the `provingKey/` (and `provingKeySnark/`) directory layout.
   This option must be run on macOS — it aborts otherwise, since the dylibs are platform-specific. The collected dylibs are later merged into the packaged proving key by option **6. Upload setup**.

5. **Build zec-reth ELF**
   Clones the `zisk-eth-client` repository (branch specified by `ZISK_ETH_CLIENT_BRANCH`) and patches its `bin/guests/stateless-validator-reth/Cargo.toml` so that the `ziskos` dependency points to the local ZisK repository resolved from `ZISK_REPO_DIR` (or `${HOME}/workspace/zisk` if unset). It then builds the guest with `cargo-zisk build --release` and verifies that `target/elf/riscv64ima-zisk-zkvm-elf/release/zec-reth` was produced.
   The resulting ELF is consumed by options **9. Test Ethereum Block** and **10. Test EthProofs**, so this option must be run before either of them.

6. **Upload setup**
   Packages the setup artifacts (`.tar.gz` + `.md5`) from the files generated in option **3. Build setup from source** (it requires `build/provingKey`): the proving key and verify key always, plus the circom circuits (`zisk-circuits`) and snark proving key (`zisk-provingkey-plonk`) when present in `build/`. When the macOS dylibs from option **4. Build dylib files (macOS)** are provided, they are merged into the proving key before packing.
   The setup input hash is computed via `setup_build.sh --print-hash`; if the bucket already holds that hash the upload is skipped (unless forced). The artifacts are stored in the `${HOME}/output` directory inside the container, which is mapped to the `./output` folder on the host, making them available externally, and are uploaded to `gs://zisk-setup` (requires `gcloud` auth).

7. **Install ZisK from binaries**
   Installs ZisK from binaries using the latest official release via `ziskup`.

8. **Test sha_hasher**
   Creates, builds, and emulates the `sha_hasher` program, then generates and verifies the proof.
   It also performs constraints verification.

9. **Test Ethereum Block**
   Tests Ethereum block proof generation using the `zec-reth` ELF and the input files cloned by option **5. Build zec-reth ELF** (which must be run beforehand).
   First, it proves the input files specified in the `BLOCK_INPUTS_SINGLE` environment variable using cargo-zisk with one single process (no mpi). Second, it proves the input files specified in the `BLOCK_INPUTS_MPI` environment variable using cargo-zisk and mpi with the number of processes and threads specified in `MPI_PROCESSES` and `MPI_THREADS` environment variables.

10. **Test EthProofs**
    Clones the `zisk-ethproofs` repository, builds it, and deploys the `zisk-coordinator` and `zisk-worker` services. Requires the `zec-reth` ELF and inputs produced by option **5. Build zec-reth ELF** (which must be run beforehand).
    Then runs the `ethproofs-client` binary against the deployed coordinator using the input files specified in `BLOCK_INPUTS_ETHPROOFS` (or `BLOCK_INPUTS_ETHPROOFS_HINTS` when `ENABLE_HINTS=1`).
    The distributed services are automatically uninstalled when the test finishes.

11. **Test ELF diagnostic**
    Runs the diagnostic ELF built from the zisk repo's `test-artifacts` crate (`test-artifacts/programs/target/elf/riscv64ima-zisk-zkvm-elf/release/diagnostic`) through the full proving pipeline using `test_elf` (verify-constraints, prove, verify) with no input file.

12. **Test docs examples**
    Builds, runs and proves the ZisK example programs under `examples/` (both the host SDK examples and the guest programs), exercising every available `{asm, gpu}` backend combination and verifying each one.

13. **Test quickstart**
    Runs the quickstart sequence of the `hash` example: builds and runs the guest program, generates its setup, proves and verifies it with every backend combination (`emulator`/`asm`, with and without `--plonk`), and finally proves it through the host SDK example. The whole set of combinations runs first on CPU and then again on GPU (adding `--gpu`) when a GPU build of `cargo-zisk` is installed; set `ONLY_CPU=1` to run just the CPU pass or `ONLY_GPU=1` to run just the GPU one. It expects ZisK, its dependencies and the proving keys / setups to be already installed.

14. **Install setup from public packages**
    Downloads and installs the proving key files from the public packages corresponding to the `ZISK_SETUP_FILE` environment variable (falling back to a name derived from the installed `cargo-zisk-dev` version when unset).

15. **Install setup from local packages**
    Installs the proving key files using the setup packages generated by option **6. Upload setup**, which must be located in the `${HOME}/output` directory.

16. **Shell**
    Opens a command line shell inside the container.
    When you exit the shell, you will return to the ZisK Test Menu.

17. **Exit**
    Exits the Release Kit container and returns to the host shell.
