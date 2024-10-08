# Install

## Requirements
* [Rust (Nightly)](https://www.rust-lang.org/tools/install)
* [xz]()
* [jq]()

### Ubuntu prerequisites
```bash
sudo apt-get install -y xz-utils jq curl git
```

## Option 1: Prebuilt Binaries (Recommended)

<div class="warning">

this is temporary until we make the repositories publics, if you need an installation token, write to the Zisk team
```bash
export ZISK_TOKEN=...
```
```bash
curl -H "Authorization: token ${ZISK_TOKEN}" \
https://raw.githubusercontent.com/0xPolygonHermez/zisk/develop/ziskup/install.sh  | bash
```
</div>





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