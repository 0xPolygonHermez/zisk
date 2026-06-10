# Installation Guide

ZisK can be installed from prebuilt binaries (recommended) or by building the ZisK tools, toolchain and setup files from source.

## System Requirements

ZisK currently supports **Linux x86_64** and **macOS** platforms (see note below).

**Note:** On **macOS**, proof generation is not yet optimized, so some proofs may take longer to generate.

### Required Tools

Ensure the following tools are installed:
* [Rust](https://www.rust-lang.org/tools/install)
* [Git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)
* [Docker](https://docs.docker.com/engine/install/) — required only on **Linux x86_64** to build ZisK from source.
* To enable GPU support in ZisK, you must have NVIDIA Driver version 525.60.13 or later installed.
* If you use the `zisk-sdk` crate, you must also have CUDA Toolkit version 12.9 or later installed.

## Installing Dependencies

### Ubuntu

Ubuntu 22.04 or higher is required.

Install all required dependencies with:
```bash
sudo apt-get install -y xz-utils jq curl build-essential qemu-system libomp-dev libgmp-dev nlohmann-json3-dev protobuf-compiler uuid-dev libgrpc++-dev libsecp256k1-dev libsodium-dev libpqxx-dev nasm libopenmpi-dev openmpi-bin openmpi-common libclang-dev clang gcc-riscv64-unknown-elf
```

ZisK uses shared memory to exchange data between processes. The system must be configured to allow enough locked memory per process:
```text
$ ulimit -l
unlimited
```
A way to achieve it is to edit the file `/etc/systemd/system.conf` and add the line `DefaultLimitMEMLOCK=infinity`. Reboot for changes to take effect.

### macOS

macOS 14 or higher is required.

You must have [Homebrew](https://brew.sh/) and [Xcode](https://developer.apple.com/xcode/) installed.

Install all required dependencies with:
```bash
brew reinstall jq curl libomp protobuf openssl nasm pkgconf open-mpi libffi nlohmann-json libsodium riscv-tools
```

## Installing ZisK

### Option 1: Prebuilt Binaries (Recommended)

1. To install ZisK using ziskup, run the following command in your terminal:
    ```bash
    curl https://raw.githubusercontent.com/0xPolygonHermez/zisk/main/ziskup/install.sh  | bash
    ```

2. During installation, ziskup will detect whether CUDA is available on your machine. If so, it will install ZisK binaries with GPU support. Otherwise, you will be prompted to choose between CPU binaries (default) or GPU binaries.

3. Also during the installation, you will be prompted to select a setup option. You can choose from the following:

    1. **Install proving key (default)** – Required for generating and verifying proofs.
    2. **Install proving key (no constant tree files)** – Install proving key but without constant tree files generation.
    3. **Install verify key** – Needed only if you want to verify proofs.
    4. **None** – Choose this if you only want to compile programs and execute them using the ZisK emulator.

4. Verify the Rust toolchain: (which includes support for the `riscv64ima-zisk-zkvm` compilation target):
    ```bash
    rustup toolchain list
    ```

    The output should include an entry for `zisk`, similar to this:
    ```
    stable-x86_64-unknown-linux-gnu (default)
    nightly-x86_64-unknown-linux-gnu
    zisk
    ```

5. Verify the `cargo-zisk` CLI tool:
    ```bash
    cargo-zisk --version
    ```

    It should show `cargo-zisk X.X.X [gpu]` if the GPU version is installed, or `cargo-zisk X.X.X [cpu]` otherwise

#### Updating ZisK

To update ZisK to the latest version, simply run:
    ```bash
    ziskup
    ```

You can use the flags `--provingkey`, `--verifykey` or `--nokey` to specify the installation setup and skip the selection prompt.

To install the PLONK proving key (provingKeySnark), run:
    ```bash
    ziskup setup_snark
    ```


### Option 2: Building from Source

#### Build ZisK

1. Clone the ZisK repository:
    ```bash
    git clone https://github.com/0xPolygonHermez/zisk.git
    cd zisk
    ```

2. Build ZisK tools:
    ```bash
    cargo build --release
    ```

    **Note**: The build process will automatically detect whether CUDA is available on your machine. If so, it will build the GPU-enabled binaries; otherwise, it will build the CPU version. To force the CPU version, use the `--features cpu-only` flag.

    **Note**: By default, the build process auto-detects the GPU architecture of the host machine. Use the `CUDA_ARCHS` environment variable to control which architectures are compiled:

    ```bash
    # Single architecture (faster build — e.g. Ada Lovelace sm_89 / RTX 4090)
    CUDA_ARCHS="89" cargo build --release

    # Multiple architectures (e.g. Ada + Hopper)
    CUDA_ARCHS="89,90" cargo build --release

    # All major architectures — portable binary for distribution
    # (sm_80, sm_86, sm_89, sm_90, sm_100, sm_120 + PTX forward compatibility)
    # Note: this takes significantly longer to compile
    CUDA_ARCHS="major" cargo build --release
    ```

3. Copy the tools to `~/.zisk/bin` directory:
    ```bash
    mkdir -p $HOME/.zisk/bin
    cp target/release/cargo-zisk target/release/ziskemu target/release/riscv2zisk target/release/zisk-coordinator target/release/zisk-worker target/release/libziskclib.a $HOME/.zisk/bin
    ```

4. Copy required files for assembly rom setup:

    **Note:** This is only needed on Linux x86_64, since assembly execution is not supported on macOS

    ```bash
    mkdir -p $HOME/.zisk/zisk/emulator-asm
    cp -r ./emulator-asm/src $HOME/.zisk/zisk/emulator-asm
    cp ./emulator-asm/Makefile $HOME/.zisk/zisk/emulator-asm
    cp -r ./lib-c $HOME/.zisk/zisk
    ```

5. Add `~/.zisk/bin` to your system PATH:

    If you are using `bash` or `zsh`:
    ```bash
    PROFILE=$([[ "$(uname)" == "Darwin" ]] && echo ".zshenv" || echo ".bashrc")
    echo >>$HOME/$PROFILE && echo "export PATH=\"\$PATH:$HOME/.zisk/bin\"" >> $HOME/$PROFILE
    source $HOME/$PROFILE
    ```

6. Install the ZisK Rust toolchain:
    ```bash
    cargo-zisk toolchain install
    ```

    **Note**: This command installs the ZisK Rust toolchain from prebuilt binaries. If you prefer to build the toolchain from source, follow these steps:

    1. Ensure all [dependencies](https://github.com/rust-lang/rust/blob/master/INSTALL.md#dependencies) required to build the Rust toolchain from source are installed.

    2. Build and install the Rust ZisK toolchain:
    ```bash
    cargo-zisk toolchain build
    ```

7. Verify the installation:
    ```bash
    rustup toolchain list
    ```
    Confirm that `zisk` appears in the list of installed toolchains.

8. Verify the `cargo-zisk` CLI tool:
    ```bash
    cargo-zisk --version
    ```

    It should show `cargo-zisk X.X.X [gpu]` if the GPU version is built, or `cargo-zisk X.X.X [cpu]` otherwise.

#### Build Setup

Please note that the process can be long, taking approximately 45-60 minutes depending on the machine used.

[NodeJS](https://nodejs.org/en/download) version 20.x or higher is required to build the setup files.

1. Clone `pil2-proofman`repository in the parent folder of the `zisk` folder created in the previous section:
    ```bash
    git clone https://github.com/0xPolygonHermez/pil2-proofman.git
    ```

2. Install packages:
    ```bash
    (cd pil2-proofman && npm install)
    ```

3. All subsequent commands must be executed from the `zisk` folder created in the previous section:
    ```bash
    cd zisk
    ```

4. Set PIL2C_EXEC environment variable
    ```bash
    export PIL2C_EXEC="../pil2-proofman/node_modules/.bin/pil2com"
    ```

5. Generate fixed data:
    ```bash
    cargo run --release --bin arith_frops_fixed_gen
    cargo run --release --bin binary_basic_frops_fixed_gen
    cargo run --release --bin binary_extension_frops_fixed_gen
    ```

6. Compile ZisK PIL:
    ```bash
    cargo-zisk proofman-setup compile-pil \
        --pil pil/zisk.pil \
        --include "pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines,precompiles" \
        --output pil/zisk.pilout \
        --fixed-dir tmp/fixed \
        --fixed-to-file \
        --no-proto-fixed-data
    ```

    This command will create the `pil/zisk.pilout` file

7. Generate pil-helpers:
    ```bash
    cargo run --release --manifest-path "../pil2-proofman/Cargo.toml" -p proofman-cli -- \
    pil-helpers \
        --pilout pil/zisk.pilout \
        --path pil/src \
        -o
    ```

8. Generate setup files:
    ```bash
    cargo-zisk proofman-setup setup \
        --airout pil/zisk.pilout \
        --build-dir $HOME/.zisk \
        --fixed-dir tmp/fixed \
        --stark-structs state-machines/starkstructs.json \
        --recursive
    ```

    This command generates the `$HOME/.zisk/provingKey` directory.

    Additionally, to generate the snark wrapper:

    ```bash
    export SNARKJS_PATH="../pil2-proofman/node_modules/snarkjs"
    cargo-zisk proofman-setup setup-snark \
        --build-dir $HOME/.zisk \
        --publics-info state-machines/publics.json \
        --powers-of-tau ../powersOfTau28_hez_final_27.ptau
    ```

    It is stored under the `$HOME/.zisk/provingKeySnark` directory.

## Uninstall Zisk

1. Uninstall ZisK toolchain:
    ```bash
    rustup uninstall zisk
    ```

2. Delete ZisK folder
    ```bash
    rm -rf $HOME/.zisk
    ```
