# Pil2-stark Lib

## Compiling locally

Steps to compile `pil2-stark` locally:
### Clone repository

```sh
git clone --recursive https://github.com/0xPolygonHermez/pil2-stark.git
cd pil2-stark
```

### Install dependencies

The following packages must be installed.

#### Ubuntu/Debian

```sh
apt update
apt install build-essential libbenchmark-dev libomp-dev libgmp-dev nlohmann-json3-dev nasm libsodium-dev cmake
```

#### openSUSE
```sh
zypper addrepo https://download.opensuse.org/repositories/network:cryptocurrencies/openSUSE_Tumbleweed/network:cryptocurrencies.repo
zypper refresh
zypper install -t pattern devel_basis
zypper install libbenchmark1 libomp16-devel libgmp10 nlohmann_json-devel nasm libsodium-devel cmake
```

#### Fedora
```
dnf group install "C Development Tools and Libraries" "Development Tools"
dnf config-manager --add-repo https://terra.fyralabs.com/terra.repo
dnf install google-benchmark-devel libomp-devel gmp gmp-devel gmp-c++ nlohmann-json-devel nasm libsodium-devel cmake
```

### Compilation

Run `make` to compile the main project:

```sh
make clean
make generate
make starks_lib -j
```


