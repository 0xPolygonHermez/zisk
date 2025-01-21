# PIL2 Proofman Toolchain
The PIL2 Proofman toolchain gives to the developer a set of CLI tools to facilitate some tasks.

These are the available set of CLI tools:

## New project

This tool creates a self-contained crate which can generate proofs from a PIL2 pilout file.

The `proofman new` CLI tool reads a `pilout` file creates a self-contained crate which can generate proofs from a PILOUT file.

```bash
proofman new --pilout <PILOUT> <NAME>
```


## Trace setup

```bash
cargo run --bin proofman-cli trace setup --pilout examples/fib4/data/fib4.pilout
```

## Pilout inspect

```bash
cargo run --bin proofman-cli pilout inspect --pilout examples/fibv/data/fibv.pilout
```

