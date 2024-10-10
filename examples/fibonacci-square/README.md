# Fibonacci Square Example Proofman Setup Guide

This guide provides step-by-step instructions for setting up the necessary repositories and executing the Fibonacci square example using the Polygon Hermez zkEVM prover.

## 1. Download and Set Up Required Repositories

### 1.2 Install `pil2-compiler`

Next, clone the `pil2-compiler` repository and install its dependencies:

```bash
git clone https://github.com/0xPolygonHermez/pil2-compiler.git
cd pil2-compiler
npm install
cd ..
```

### 1.3 Install `pil2-proofman-js`

Clone the `pil2-proofman-js` repository, switch to the `feature/setup` branch, and install the dependencies:

```bash
git clone https://github.com/0xPolygonHermez/pil2-proofman-js
cd pil2-proofman-js
git checkout feature/setup

# TODO: Verify if the Stark Recurser raises any issues during this process

npm install
cd ..
```

### 1.4 Install `zkevm-prover`

Clone the `zkevm-prover` repository, switch to the `develop_rust_lib` branch, and install the necessary system dependencies:

```bash
git clone https://github.com/0xPolygonHermez/zkevm-prover.git
cd zkevm-prover
git checkout develop_rust_lib

# Update package lists and install required system packages
sudo apt update
sudo apt install -y build-essential libbenchmark-dev libomp-dev libgmp-dev nlohmann-json3-dev nasm libsodium-dev cmake

# Initialize and update git submodules
git submodule init
git submodule update

# Clean previous builds and compile the starks library
make clean
make starks_lib -j
cd ..
```

### 1.5 Install `pil2-proofman`

Finally, clone the `pil2-proofman` repository:

```bash
git clone https://github.com/0xPolygonHermez/pil2-proofman.git
cd pil2-proofman
```

---

## 2. Execute the Fibonacci Square Example

### 2.1 Compile PIL

To begin, compile the PIL files:

```bash
node ../pil2-compiler/src/pil.js ./examples/fibonacci-square/pil/build.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./examples/fibonacci-square/pil/build.pilout
```

### 2.2 Generate Setup

After compiling the PIL files, generate the setup:

```bash
node ../pil2-proofman-js/src/main_setup.js \
     -a ./examples/fibonacci-square/pil/build.pilout \
     -b ./examples/fibonacci-square/build
```

### 2.3 Generate PIL Helpers

Generate the corresponding PIL helpers by running the following command:

```bash
cargo run --bin proofman-cli pil-helpers \
     --pilout ./examples/fibonacci-square/pil/build.pilout \
     --path ./examples/fibonacci-square/src -o
```

### 2.4 Build the Project

Build the project with the following command:

```bash
cargo build
```

### 2.5 Verify Constraints

Verify the constraints by executing this command:

```bash
cargo run --bin proofman-cli verify-constraints \
     --witness-lib ./target/debug/libfibonacci_square.so \
     --proving-key examples/fibonacci-square/build/provingKey/ \
     --public-inputs examples/fibonacci-square/src/inputs.json
```

### 2.6 Generate Proof

Finally, generate the proof using the following command:

```bash
cargo run --bin proofman-cli prove \
     --witness-lib ./target/debug/libfibonacci_square.so \
     --proving-key examples/fibonacci-square/build/provingKey/ \
     --public-inputs examples/fibonacci-square/src/inputs.json \
     --output-dir examples/fibonacci-square/build/proofs -d
```

### 2.7 Verify the Proof

```bash
node ../pil2-proofman-js/src/main_verify -k examples/fibonacci-square/build/provingKey/ -p examples/fibonacci-square/build/proofs
```

### 2.6 Generate Final Proof

Finally, generate the proof using the following command:

```bash
cargo run --bin proofman-cli prove \
     --witness-lib ./target/debug/libfibonacci_square.so \
     --proving-key examples/fibonacci-square/build/provingKey/ \
     --public-inputs examples/fibonacci-square/src/inputs.json \
     --output-dir examples/fibonacci-square/build/proofs \
     -a
```

### 2.8 Verify final proof

```bash
node ../pil2-stark-js/src/main_verifier.js -v examples/fibonacci-square/build/provingKey/build/final/final.verkey.json -s examples/fibonacci-square/build/provingKey/build/final/final.starkinfo.json -i examples/fibonacci-square/build/provingKey/build/final/final.verifierinfo.json -o examples/fibonacci-square/build/proofs/proofs/final_proof.json -b examples/fibonacci-square/build/proofs/publics.json
```