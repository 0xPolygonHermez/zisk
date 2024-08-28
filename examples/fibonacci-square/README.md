This is the fibonacci vadcop example.
To use it from scratch you need to install pil2-compiler and pil2-components and pil2-proofman-js (feature/setup branch)

Firstly, the PIL files needs to be compiled 

```
node ../pil2-compiler/src/pil.js ../pil2-components/test/fibonacci/pil/build.pil -I ../pil2-components/lib/std/pil -o ./examples/fibonacci-square/pil/build.pilout
```

When compilation is done, the setup is generated using the following command


```
node ../pil2-proofman-js/src/setup/main_genSetup.js -a ./examples/fibonacci-square/pil/build.pilout -b ./examples/fibonacci-square/build
```


After that, generate the corresponding pil-helpers:

```
cargo run --bin proofman-cli pil-helpers --pilout ./examples/fibonacci-square/pil/build.pilout --path ./examples/fibonacci-square/src -o
```

Build the project:

```
cargo build
```

Constraints can be verified running the following command

```
cargo run --bin proofman-cli verify-constraints --witness-lib ./target/debug/libfibonacci_square.so --proving-key examples/fibonacci-square/build/provingKey/ --public-inputs examples/fibonacci-square/src/inputs.json
```

To generate proof:

```
cargo run --bin proofman-cli prove --witness-lib ./target/debug/libfibonacci_square.so --proving-key examples/fibonacci-square/build/provingKey/ --public-inputs examples/fibonacci-square/src/inputs.json
```

Finally, to verify this proof:

```
TODO!
```