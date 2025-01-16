# Installation Guide

You can install the ZisK toolchain either from prebuilt binaries (recommended) or by building it from source.

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
You can also use a custom shell: 
    ```bash
    nix develop -c zsh
    ```

    This will open a shell with the `PATH` and `LD_LIBRARY_PATH` correctly configured for building the project. Exit the shell with `Ctrl+D`.

## Installing ZisK toolchain

### Option 1: Prebuilt binaries (recommended)
1. Install the ZisK toolchain installer `ziskup`:
    ```bash
    curl https://raw.githubusercontent.com/0xPolygonHermez/zisk/develop/ziskup/install.sh  | bash
    ```

    This will enable the `ziskup` command in your terminal. Restart your terminal session or run:
    ```bash
    source $HOME/.bashrc
    ```

2. Use `ziskup` to install the ZisK toolchain:
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

### Option 2: Building from source
1. Ensure all [dependencies](https://github.com/rust-lang/rust/blob/master/INSTALL.md#dependencies) required to build the Rust toolchain from source are installed.

2. Clone the ZisK repository:
    ```bash
    git clone git@github.com:0xPolygonHermez/zisk.git
    cd zisk
    ```

3. Build the ZisK toolchain:
    ```bash
    cargo run --bin=cargo-zisk -- sdk build-toolchain
    ```

4. Install the ZisK toolchain:
    ```bash
    cargo run --bin=cargo-zisk -- sdk install-toolchain
    ```

5. Verify the installation:
    ```bash
    rustup toolchain list
    ```
    Ensure `zisk` appears in the list of installed toolchains.

## Update ZisK toolchain
To update the ZisK toolchain to the latest version, simply run:
```bash
ziskup
```

## Uninstall Zisk toolchain
To uninstall the ZisK toolchain run:
```bash
rustup toolchain remove zisk
```
