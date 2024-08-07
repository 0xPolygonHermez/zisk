# Quickstart

This guide will show you how to compile zisk-wc library and create all the staff needed to generate a proof.

## Requirements

Before starting, make sure you have [Rust](https://www.rust-lang.org/tools/install) installed on your system.

Optional recommendations:

- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extension if you are using VS Code to assist you when writing Rust code.
- [PIL2 Highlight syntax code](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) if you are using VS Code to highlight your code when writing PIL2 code.

Install the following repositories:

```bash
git clone https://github.com/0xPolygonHermez/pil2-compiler.git
git clone https://github.com/0xPolygonHermez/pil2-proofman-js.git
git clone https://github.com/0xPolygonHermez/pil2-components.git
git clone https://github.com/0xPolygonHermez/pil2-proofman.git
```

## Compile the PIL files to generate a PILOUT

Compiling the PIL using the [PIL2 compiler repository](https://github.com/0xPolygonHermez/pil2-compiler.git) you generate a PILOUT file. Compile the PIL2 compiler by running the following commands:

```bash
node ../pil2-compiler/src/pil.js ./zkevm/zisk-wc/pil/zisk.pil -I ../pil2-components/lib/std/pil
```

This command will generate a `zisk.pilout` file that contains the arithmetization, public inputs, constraints, constant values, and other proof-generation-specific details described by the PIL2 project.

## Generate the setup files

```
node ../pil2-proofman-js/src/setup/main_genSetup.js -a ./zkvm/zisk-wc/pil/zisk.pilout -s ./zkevm/zisk-wc/setup/stark_structs.json -b ./zkvm/zisk-wc/setup
/build
```

## Compile the dynamic library

```
cd zisk-wc
cargo build

```

## Launch the proof generation

```
cd ../pil2-proofman
cargo run --bin proofman-cli prove --wc-lib ../zisk/target/debug/libzisk_wc.dylib --proving-key ../zisk/zkvm/zisk-wc/setup
/build/provingKey --public-inputs ../zisk/zkvm/zisk-wc/inputs/inputs.hex
```



        // fn execute(&self, pctx: &mut ProofCtx<F>, wneeds: &WitnessNeeds) {
        // Creates the ectx with the workers pool inside
        // TODO let mut ectx = self.wcm.createExecutionContext(wneeds);
        self.main_sm.execute(pctx, ectx);
        // TODO ectx.terminate();
