# Install



## Requirements
⚠️ Currently, macOS is not supported for proof generation. A Linux x86_64 machine is required at this time. ⚠️

* [Rust](https://www.rust-lang.org/tools/install)
* [xz]()
* [jq]()

### Ubuntu prerequisites
```bash
sudo apt-get install -y xz-utils jq curl git build-essential qemu-system libomp-dev libgmp-dev nlohmann-json3-dev protobuf-compiler uuid-dev libgrpc++-dev libsecp256k1-dev libsodium-dev libpqxx-dev nasm
```

### Nix Flake

Alternatively, you can use [Nix package manager](https://github.com/NixOS/nix) to install all dependencies. First, follow the [guide to install Nix](https://determinate.systems/nix/) on your OS.

Afterwards, use `flake.nix` in `zisk` repository to load the development environment with:
```
nix develop

# You can also use a custom shell: 
nix develop -c zsh
```

This will start a new shell with correctly set `PATH` and `LD_LIBRARY_PATH` for dependencies necessary to build the project.
You can exit this shell with Ctrl+D.

### OSX prerequisites
```bash
# Install brew first.
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# A dependency for `cargo build`.
brew install protobuf

# A dependency of `ziskup`.
brew install libusb jq
```

#### nodejs
```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
source $HOME/.bashrc
nvm install 19
nvm use 19
```

#### Circom
```bash
https://docs.circom.io/getting-started/installation/
```

## Option 1: Prebuilt Binaries (Recommended)


```bash
curl https://raw.githubusercontent.com/0xPolygonHermez/zisk/develop/ziskup/install.sh  | bash
```

This will enable the ziskup command in your CLI. You need to restart your terminal to use it or run this command:

```bash
source $HOME/.bashrc
```

After completing these steps, you can execute ziskup to install the toolchain:

```bash
ziskup
```


To check the correct installation of the ZisK Rust toolchain which has support for the riscv64ima-polygon-ziskos-elf compilation target, you can run the next command
```bash
$ rustup toolchain list
stable-x86_64-unknown-linux-gnu
nightly-2024-01-25-x86_64-unknown-linux-gnu (default)
nightly-2024-03-05-x86_64-unknown-linux-gnu
nightly-2024-06-30-x86_64-unknown-linux-gnu
nightly-x86_64-unknown-linux-gnu
zisk
```
