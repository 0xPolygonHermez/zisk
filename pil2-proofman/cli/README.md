# PROOFMAN Toolchain
Proofman provides a toolchain for helping in programming zero knowledge proof applications. 

Following are the commands of the toolchain:

## Commands

### New. Creates a new project
New command creates a new project with the given name. It creates a new directory with the given name with crate with the same name. The crate contains the witness computation modules as well as the proof main generation module. All the data to create the project is extracted from the pilout file.

```
cargo run --bin proofman-cli new fib_module --pilout examples/fibv/data/fibv.pilout
```

### Pilout inspect. Inspects the pilout file

```
cargo run --bin proofman-cli pilout inspect --pilout examples/fibv/data/fibv.pilout
```

### Trace setup. Creates the traces defined in the pilout file
```
 cargo run --bin proofman-cli trace setup --pilout examples/fib4/data/fib4.pilout
 ``````