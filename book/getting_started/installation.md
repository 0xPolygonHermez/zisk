# Installation Guide

ZisK can be installed from prebuilt binaries (recommended) or by building ZisK tools, toolchain and setup files from source.

## System Requirements

ZisK currently supports Linux x86_64 systems. **Proof generation on macOS is not supported.**

### Required Tools (Linux & macOS)

Ensure the following tools are installed:
* [Rust](https://www.rust-lang.org/tools/install)
* [Git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)

## Installing Dependencies

### Ubuntu

Ubuntu 22.04 or higher is required.

Install all required dependencies with:
```bash
sudo apt-get install -y xz-utils jq curl build-essential qemu-system libomp-dev libgmp-dev nlohmann-json3-dev protobuf-compiler uuid-dev libgrpc++-dev libsecp256k1-dev libsodium-dev libpqxx-dev nasm
```

### macOs

1. Install Homebrew:
    ```bash
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    ```

2. Install protobuf (required for `cargo build`):
    ```bash
    brew install protobuf
    ```

3. Install libusb & jq (required for `ziskup`):
    ```bash
    brew install libusb jq
    ```

4. Install Node.js:
    ```bash
    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
    source $HOME/.bashrc
    nvm install 19
    nvm use 19
    ```

5. Install Circom:
    ```bash
    https://docs.circom.io/getting-started/installation/
    ```

### Alternative: Using Nix Flake

You can use [Nix](https://github.com/NixOS/nix) to install all dependencies.

1. Follow the instructions to install [Nix](https://determinate.systems/nix/) on your system.

2. Use the `flake.nix` file from the [ZisK repository](https://github.com/0xPolygonHermez/zisk/blob/main/flake.nix) to set up the development environment:
    ```bash
    nix main
    ```

3. To start a shell with ZisK’s environment:
    ```bash
    nix main -c zsh
    ```
    This will open a shell with the `PATH` and `LD_LIBRARY_PATH` correctly configured for building the project. Exit the shell with `Ctrl+D`.

## Installing ZisK

### Option 1: Prebuilt Binaries (Recommended)

1. Install the ZisK installer `ziskup`:
    ```bash
    curl https://raw.githubusercontent.com/0xPolygonHermez/zisk/main/ziskup/install.sh  | bash
    ```
    This will enable the `ziskup` command in your terminal. 
    
    Restart your terminal session or run:
    ```bash
    source $HOME/.bashrc
    ```

2. Install the ZisK toolchain and CLI tools:
    ```bash
    ziskup
    ```

3. Verify the Rust toolchain: (which includes support for the `riscv64ima-polygon-ziskos` compilation target):
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

5. Download and install setup files: 

    Option 1:
    Download the proving key files:
    ```bash
    curl -O https://storage.googleapis.com/zisk/zisk-provingkey-0.4.0.tar.gz
    curl -O https://storage.googleapis.com/zisk/zisk-provingkey-0.4.0.tar.gz.md5
    ```

    Verify the MD5 checksum:
    ```bash
    md5sum -c zisk-provingkey-0.4.0.tar.gz.md5
    ```

    Extract the file to the `$HOME/.zisk` directory:
    ```bash
    tar --overwrite -xvf zisk-provingkey-0.4.0.tar.gz -C $HOME/.zisk
    ```

    Option 2:
    Alternatively, if you only need to verify proofs, download and install the verify key files:
     ```bash
    curl -O https://storage.googleapis.com/zisk/zisk-verifykey-0.4.0.tar.gz
    curl -O https://storage.googleapis.com/zisk/zisk-verifykey-0.4.0.tar.gz.md5
    ```    

    Then, follow the same verification and installation steps as for the proving key files.

To update ZisK to the latest version, simply run again the previous steps.

### Option 2: Building from Source

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
    cp target/release/cargo-zisk target/release/ziskemu target/release/riscv2zisk target/release/libzisk_witness.so precompiles/keccakf/src/keccakf_script.json $HOME/.zisk/bin
    ```

5. Add `~/.zisk/bin` to your profile file, for example for `.bashrc` executing the following commands:
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
    (cd pil2-compiler && npm i)
    (cd pil2-proofman-js && npm i)

3. **Note:** All subsequent commands must be executed from the `zisk` folder created in the previous section.

4. Adjust memory mapped areas and JavaScript heap size:
    ```bash
    echo "vm.max_map_count=655300" | sudo tee -a /etc/sysctl.conf
    sudo sysctl -w vm.max_map_count=655300
    export NODE_OPTIONS="--max-old-space-size=230000"
    ```

4. Compile ZisK PIL: (Note that this command may take 20-30 minutes to complete)
    ```bash
    node --max-old-space-size=131072 ../pil2-compiler/src/pil.js pil/zisk.pil -I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines,precompiles -o pil/zisk.pilout
    ```

    This command will create the `pil/zisk.pilout` file

5. Generate fixed data:
    ```bash
    cargo run --release --bin keccakf_fixed_gen
    mkdir -p build
    mv precompiles/keccakf/src/keccakf_fixed.bin build
    ```

    These commands generates the `keccakf_fixed.bin` file in the `build` directory.

6. Generate setup data: (Note that this command may take 2–3 hours to complete):
    ```bash
    node --max-old-space-size=65536 ../pil2-proofman-js/src/main_setup.js -a ./pil/zisk.pilout -b build -i ./build/keccakf_fixed.bin -r
    ```

    This command generates the `provingKey` directory.

7. Copy (or move) the `provingKey` directory to `$HOME/.zisk` directory:

    ```bash
    cp -R build/provingKey $HOME/.zisk
    ```

## Uninstall Zisk
To uninstall ZisK, run:

```bash
rm -rf $HOME/.zisk
```
