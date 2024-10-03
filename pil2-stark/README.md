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

**Important dependency note**: you must install [`libpqxx` version 6.4.5](https://github.com/jtv/libpqxx/releases/tag/6.4.5). If your distribution installs a newer version, please [compile `libpqxx` 6.4.5](https://github.com/jtv/libpqxx/releases/tag/6.4.5) and install it manually instead.

#### Ubuntu/Debian

```sh
apt update
apt install build-essential libbenchmark-dev libomp-dev libgmp-dev nlohmann-json3-dev postgresql libpqxx-dev libpqxx-doc nasm libsecp256k1-dev grpc-proto libsodium-dev libprotobuf-dev libssl-dev cmake libgrpc++-dev protobuf-compiler protobuf-compiler-grpc uuid-dev
```

#### openSUSE
```sh
zypper addrepo https://download.opensuse.org/repositories/network:cryptocurrencies/openSUSE_Tumbleweed/network:cryptocurrencies.repo
zypper refresh
zypper install -t pattern devel_basis
zypper install libbenchmark1 libomp16-devel libgmp10 nlohmann_json-devel postgresql libpqxx-devel ghc-postgresql-libpq-devel nasm libsecp256k1-devel grpc-devel libsodium-devel libprotobuf-c-devel libssl53 cmake libgrpc++1_57 protobuf-devel uuid-devel llvm llvm-devel libopenssl-devel
```

#### Fedora
```
dnf group install "C Development Tools and Libraries" "Development Tools"
dnf config-manager --add-repo https://terra.fyralabs.com/terra.repo
dnf install google-benchmark-devel libomp-devel gmp gmp-devel gmp-c++ nlohmann-json-devel postgresql libpqxx-devel nasm libsecp256k1-devel grpc-devel libsodium-devel cmake grpc grpc-devel grpc-cpp protobuf-devel protobuf-c-devel uuid-devel libuuid-devel uuid-c++ llvm llvm-devel openssl-devel 
```

### Compilation

Run `make` to compile the main project:

```sh
make clean
make generate
make starks_lib -j
```


