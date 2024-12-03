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

Clone the `pil2-proofman-js` repository, switch to the `develop` branch, and install the dependencies:

```bash
git clone https://github.com/0xPolygonHermez/pil2-proofman-js
cd pil2-proofman-js
git checkout develop

# TODO: Verify if the Stark Recurser raises any issues during this process

npm install
cd ..
```
# Update package lists and install required system packages
sudo apt update
sudo apt install -y build-essential libbenchmark-dev libomp-dev libgmp-dev nlohmann-json3-dev nasm libsodium-dev cmake

### Compile the PIl2 Stark C++ Library (run only once):
```bash
(cd ../pil2-proofman/pil2-stark && git submodule init && git submodule update && make clean && make -j starks_lib && make -j bctree)
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
     -t ./pil2-stark/build/bctree
```

To run the aggregated proof, need to add -r to the previous command

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
     --output-dir examples/fibonacci-square/build/proofs
```

### 2.7 Verify the Proof

```bash
node ../pil2-proofman-js/src/main_verify -k examples/fibonacci-square/build/provingKey/ -p examples/fibonacci-square/build/proofs
```

### 2.6 Generate VadcopFinal Proof

This will only work if setup is generated with -r
Finally, generate the final proof using the following command:

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
node ../pil2-proofman-js/src/main_verify -k examples/fibonacci-square/build/provingKey/ -p examples/fibonacci-square/build/proofs -t vadcop_final
```

### 2.9 All at once

```bash
node ../pil2-compiler/src/pil.js ./examples/fibonacci-square/pil/build.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./examples/fibonacci-square/pil/build.pilout \
&& node ../pil2-proofman-js/src/main_setup.js \
     -a ./examples/fibonacci-square/pil/build.pilout \
     -b ./examples/fibonacci-square/build \
     -t ./pil2-stark/build/bctree \
&& cargo run --bin proofman-cli pil-helpers \
     --pilout ./examples/fibonacci-square/pil/build.pilout \
     --path ./examples/fibonacci-square/src -o \
&& cargo build \
&& cargo run --bin proofman-cli verify-constraints \
     --witness-lib ./target/debug/libfibonacci_square.so \
     --proving-key examples/fibonacci-square/build/provingKey/ \
     --public-inputs examples/fibonacci-square/src/inputs.json \
&& cargo run --bin proofman-cli prove \
     --witness-lib ./target/debug/libfibonacci_square.so \
     --proving-key examples/fibonacci-square/build/provingKey/ \
     --public-inputs examples/fibonacci-square/src/inputs.json \
     --output-dir examples/fibonacci-square/build/proofs\
&& node ../pil2-proofman-js/src/main_verify -k examples/fibonacci-square/build/provingKey/ -p examples/fibonacci-square/build/proofs
```
