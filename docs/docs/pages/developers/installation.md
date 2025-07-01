---
description: Guide developers through the installation process
---

# Installation Guide

ZisK can be installed from prebuilt binaries (recommended) or by building the ZisK tools, toolchain and setup files from source.

## System Requirements

ZisK currently supports **Linux x86_64** and **macOS** platforms (see note below).

>**Note:** Proof generation and verification on **macOS** are not yet supported. We’re actively working to add this functionality.

### Required Tools

Ensure the following tools are installed:
* [Rust](https://www.rust-lang.org/tools/install)
* [Git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)

## Installing Dependencies

### Ubuntu

Ubuntu 22.04 or higher is required.

Install all required dependencies with:
```bash
sudo apt-get install -y xz-utils jq curl build-essential qemu-system libomp-dev libgmp-dev nlohmann-json3-dev protobuf-compiler uuid-dev libgrpc++-dev libsecp256k1-dev libsodium-dev libpqxx-dev nasm libopenmpi-dev openmpi-bin openmpi-common libclang-dev clang
```

### macOS

macOS 14 or higher is required.

You must have [Homebrew](https://brew.sh/) installed.

Install all required dependencies with:
```bash
brew reinstall jq curl libomp protobuf openssl nasm pkgconf open-mpi libffi
```

## Installing ZisK

### Option 1: Prebuilt Binaries (Recommended)

1. To install ZisK using ziskup, run the following command in your terminal:
    ```bash
    curl https://raw.githubusercontent.com/0xPolygonHermez/zisk/main/ziskup/install.sh  | bash
    ```

2. During the installation, you will be prompted to select a setup option. You can choose from the following:

    1. **Install proving key (default)** – Required for generating and verifying proofs.
    2. **Install verify key** – Needed only if you want to verify proofs.
    3. **None** – Choose this if you only want to compile programs and execute them using the ZisK emulator.

3. Verify the Rust toolchain: (which includes support for the `riscv64ima-zisk-zkvm` compilation target):
    ```bash
    rustup toolchain list
    ```

    The output should include an entry for `zisk`, similar to this:
    ```
    stable-x86_64-unknown-linux-gnu (default)
    nightly-x86_64-unknown-linux-gnu
    zisk
    ```

4. Verify the `cargo-zisk` CLI tool:
    ```bash
    cargo-zisk --version
    ```

#### Updating ZisK

To update ZisK to the latest version, simply run:
    ```bash
    ziskup
    ```

You can use the flags `--provingkey`, `--verifykey` or `--nokey` to specify the installation setup and skip the selection prompt.


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

    **Note**: If you encounter the following error during compilation:
    ```
    --- stderr
    /usr/lib/x86_64-linux-gnu/openmpi/include/mpi.h:237:10: fatal error: 'stddef.h' file not found
    ```

    Follow these steps to resolve it:

    1. Locate the `stddef.h` file:
        ```bash
        find /usr -name "stddef.h"
        ```
    2. Set the environment variables to include the directory where `stddef.h` is located (e.g.):
        ```bash
        export C_INCLUDE_PATH=/usr/lib/gcc/x86_64-linux-gnu/13/include
        export CPLUS_INCLUDE_PATH=$C_INCLUDE_PATH
        ```
    3. Try building again        

3. Copy the tools to `~/.zisk/bin` directory:
    ```bash
    mkdir -p $HOME/.zisk/bin
    cp target/release/cargo-zisk target/release/ziskemu target/release/riscv2zisk target/release/libzisk_witness.so precompiles/sha256f/src/sha256f_script.json $HOME/.zisk/bin
    ```

4. Copy required files to support `cargo-zisk rom-setup` command:
    ```bash
    mkdir -p $HOME/.zisk/zisk/emulator-asm
    cp -r ./emulator-asm/src $HOME/.zisk/zisk/emulator-asm
    cp ./emulator-asm/Makefile $HOME/.zisk/zisk/emulator-asm
    cp -r ./lib-c $HOME/.zisk/zisk
    ```

5. Add `~/.zisk/bin` to your system PATH:
    For example, if you are using `bash`:
    ```bash
    echo >>$HOME/.bashrc && echo "export PATH=\"\$PATH:$HOME/.zisk/bin\"" >> $HOME/.bashrc
    source $HOME/.bashrc
    ```

6. Install the ZisK Rust toolchain:
    ```bash
    cargo-zisk sdk install-toolchain
    ```

    **Note**: This command installs the ZisK Rust toolchain from prebuilt binaries. If you prefer to build the toolchain from source, follow these steps:

    1. Ensure all [dependencies](https://github.com/rust-lang/rust/blob/master/INSTALL.md#dependencies) required to build the Rust toolchain from source are installed.

    2. Build and install the Rust ZisK toolchain:
    ```bash
    cargo-zisk sdk build-toolchain
    ```

7. Verify the installation:
    ```bash
    rustup toolchain list
    ```
    Confirm taht `zisk` appears in the list of installed toolchains.

#### Build Setup

The setup building process is highly intensive in terms of CPU and memory usage. You will need a machine with at least the following hardware requirements:

* 32 CPUs
* 512 GB of RAM
* 100 GB of free disk space

Please note that the process can be long, taking approximately 2–3 hours depending on the machine used.

[NodeJS](https://nodejs.org/en/download) version 20.x or higher is required to build the setup files.

1. Clone the following repositories in the parent folder of the `zisk` folder created in the previous section:
    ```bash
    git clone https://github.com/0xPolygonHermez/pil2-compiler.git
    git clone https://github.com/0xPolygonHermez/pil2-proofman.git
    git clone https://github.com/0xPolygonHermez/pil2-proofman-js
    ```
2. Install packages:
    ```bash
    (cd pil2-compiler && npm i)
    (cd pil2-proofman-js && npm i)

3. All subsequent commands must be executed from the `zisk` folder created in the previous section:
    ```bash
    cd ~/zisk
    ```

4. Adjust memory mapped areas and JavaScript heap size:
    ```bash
    echo "vm.max_map_count=655300" | sudo tee -a /etc/sysctl.conf
    sudo sysctl -w vm.max_map_count=655300
    export NODE_OPTIONS="--max-old-space-size=230000"
    ```

5. Compile ZisK PIL: (Note that this command may take 20-30 minutes to complete)
    ```bash
    node --max-old-space-size=131072 ../pil2-compiler/src/pil.js pil/zisk.pil -I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines,precompiles -o pil/zisk.pilout
    ```

    This command will create the `pil/zisk.pilout` file

6. Generate fixed data:
    ```bash
    cargo run --release --bin keccakf_fixed_gen
    cargo run --release --bin sha256f_fixed_gen
    mkdir -p build
    mv precompiles/keccakf/src/keccakf_fixed.bin build 
    mv precompiles/sha256f/src/sha256f_fixed.bin build
    ```

    These commands generate the `keccakf_fixed.bin` and `sha256f_fixed.bin` files in the `build` directory.

7. Generate setup data: (Note that this command may take 2–3 hours to complete):
    ```bash
    node --max-old-space-size=131072 ../pil2-proofman-js/src/main_setup.js -a ./pil/zisk.pilout -b build -i ./build/keccakf_fixed.bin ./build/sha256f_fixed.bin -r
    ```

    This command generates the `provingKey` directory.

8. Copy (or move) the `provingKey` directory to `$HOME/.zisk` directory:

    ```bash
    cp -R build/provingKey $HOME/.zisk
    ```

9. Generate constant tree files:
    ```bash
    cargo-zisk check-setup -a
    ```

## Uninstall Zisk

1. Uninstall ZisK toolchain:
    ```bash
    rustup uninstall zisk
    ```

2. Delete ZisK folder
    ```bash
    rm -rf $HOME/.zisk
    ```