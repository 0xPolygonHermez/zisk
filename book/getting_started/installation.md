# Installation Guide

You can install ZisK from prebuilt binaries (recommended) or by building ZisK tools, toolchain and setup files from source.

## Requirements
ZisK currently supports Linux x86_64 systems. **Proof generation on macOS is not supported.**

The following tools are required on both Linux and macOS:
* [Rust](https://www.rust-lang.org/tools/install)
* [Git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)

### Ubuntu
Run the following command to install all necessary packages and dependencies for ZisK:
```bash
sudo apt-get install -y xz-utils jq curl build-essential qemu-system libomp-dev libgmp-dev nlohmann-json3-dev protobuf-compiler uuid-dev libgrpc++-dev libsecp256k1-dev libsodium-dev libpqxx-dev nasm
```

### macOs
Follow these steps to install all the necessary packages and dependencies for ZisK:

1. Install `brew`:
    ```bash
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    ```

2. Install `protobuf` (required for building the project with `cargo build`)
    ```bash
    brew install protobuf
    ```

3. Install `libusb` and `jq` (required for `ziskup`)
    ```bash
    brew install libusb jq
    ```

4. Install `nodejs`:
    ```bash
    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
    source $HOME/.bashrc
    nvm install 19
    nvm use 19
    ```

5. Install `circom`:
    ```bash
    https://docs.circom.io/getting-started/installation/
    ```

### Using Nix Flake

As an alternative, you can use the [Nix package manager](https://github.com/NixOS/nix) to install all dependencies.

1. Follow the instructions to install [Nix]https://determinate.systems/nix/) on your system.

2. Use the [`flake.nix`]([ZisK repository](https://github.com/0xPolygonHermez/zisk/blob/develop/flake.nix) file in the ZisK repository to set up the development environment:
    ```bash
    nix develop
    ```
3. To start a custom shell, use:
    ```bash
    nix develop -c zsh
    ```

    This will open a shell with the `PATH` and `LD_LIBRARY_PATH` correctly configured for building the project. Exit the shell with `Ctrl+D`.

## Installing ZisK

### Option 1: Prebuilt binaries (recommended)
1. Install the ZisK installer `ziskup`:
    ```bash
    curl https://raw.githubusercontent.com/0xPolygonHermez/zisk/develop/ziskup/install.sh  | bash
    ```

    This will enable the `ziskup` command in your terminal. Restart your terminal session or run:
    ```bash
    source $HOME/.bashrc
    ```

2. Use `ziskup` to install ZisK toolchain and CLI tools:
    ```bash
    ziskup
    ```

3. Verify the installation of the ZisK Rust toolchain (which includes support for the `riscv64ima-polygon-ziskos` compilation target):
    ```bash
    rustup toolchain list
    ```
    The output should include an entry for `zisk`, similar to this:
    ```
    stable-x86_64-unknown-linux-gnu (default)
    nightly-x86_64-unknown-linux-gnu
    zisk
    ```

4. Verify the installation of `cargo-zisk` CLI tool:
    ```bash
    cargo-zisk --version
    ```

5. Download and install setup files:

To update ZisK to the latest version, simply run again:

```bash
ziskup
```

### Option 2: Building from source

#### Build ZisK

1. Ensure all [dependencies](https://github.com/rust-lang/rust/blob/master/INSTALL.md#dependencies) required to build the Rust toolchain from source are installed.

2. Clone the ZisK repository:
    ```bash
    git clone https://github.com/0xPolygonHermez/zisk.git
    cd zisk
    ```

3. Build ZisK tools:
    ```bash
    cargo build --release
    ```

4. Copy the tools to `~/.zisk/bin` directory:
    ```bash
    mkdir -p $HOME/.zisk/bin
    cp target/release/cargo-zisk target/release/ziskemu target/release/riscv2zisk target/release/libzisk_witness.so $HOME/.zisk/bin
    ```

5. Add `~/.zisk/bin` to your profile file, for example for `.bashrc` execute the following commands:
    ```bash
    echo >>$HOME/.bashrc && echo "export PATH=\"\$PATH:$HOME/.zisk/bin\"" >> $HOME/.bashrc
    source $HOME/.bashrc
    ```

6. Build the Rust ZisK toolchain:
    ```bash
    cargo-zisk sdk build-toolchain
    ```

7. Install the Rust ZisK toolchain:
    ```bash
    ZISK_TOOLCHAIN_SOURCE_DIR=. cargo-zisk sdk install-toolchain
    ```

8. Verify the installation:
    ```bash
    rustup toolchain list
    ```
    Ensure `zisk` appears in the list of installed toolchains.

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
    (cd ../pil2-compiler && npm i)
    (cd ../pil2-proofman-js && npm i)

3. **Note:** All subsequent commands must be executed from the `zisk` folder created in the previous section.

4. Compile ZisK PIL: (Note that this command may take 30-40 minutes to complete)
    ```bash
    node --max-old-space-size=131072 ../pil2-compiler/src/pil.js pil/zisk.pil -I pil, ../pil2-proofman/pil2-components/lib/std/pil, state-machines, precompiles -o pil/zisk.pilout
    ```

    This command will create the `pil/zisk.pilout` file

5. Generate fixed data:
    ```bash
    cargo run --release --bin keccakf_fixed_gen
    mkdir build
    mv precompiles/keccakf/src/keccakf_fixed.bin build
    ```

    These commands generates the `keccakf_fixed.bin` file in the `build` directory.

6. Generate setup data: (Note that this command may take 2–3 hours to complete)
    ```bash
    node --max-old-space-size=65536 ../pil2-proofman-js/src/main_setup.js -a ./pil/zisk.pilout -b build -i ./build/keccakf_fixed.bin -r
    ```

    This command generates the `provingKey` directory.

7. Copy (or move) the `provingKey` directory to `$HOME/.zisk` directory:

    ```bash
    cp -R provingKey $HOME/.zisk
    ```

## Uninstall Zisk toolchain
To uninstall the ZisK toolchain run:
```bash
rustup toolchain remove zisk
```
