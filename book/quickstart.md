# Quickstart

In this section we will guide you through the process of creating a simple example using PIL2 Proofman. We will use the Fibonacci sequence as an example.

## Requirements

Before starting, make sure you have [Rust](https://www.rust-lang.org/tools/install) installed on your system.

Optional recommendations:

- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extension if you are using VS Code to assist you when writing Rust code.
- [PIL2 Highlight syntax code](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) if you are using VS Code to highlight your code when writing PIL2 code.

Install the following repositories:

```bash
git clone https://github.com/0xPolygonHermez/pil2-compiler.git
git clone https://github.com/0xPolygonHermez/pil2-proofman.git
```

## PIL2 code
Let's start by creating a simple PIL2 code that calculates the Fibonacci sequence. Create a new file named `fibonacci.pil` with the following content:

```pil2
public in1;
public in2;
public out;

airgroup Fibonacci(2**10) {
    col witness a,b;

    col fixed L1 = [1,0...];
    col fixed LLAST = [0...,1];

    (b' - a) * (1 - L1') === 0;

    L1 * (b - in1) === 0;
    L1 * (a - in2) === 0;
    LLAST * (a - out) === 0;
}
```

In this code you are defining a PIL2 with a single airgroup inside it. The airgroup calculates the Fibonacci sequence and is defined to compute a unique type of Algebraic Intermediate Representation (from now on AIR) of 2<sup>10</sup> rows. The airgroup has three public inputs `in1`, `in2` and `out`, has two witness columns `a` and `b` and two fixed (a.k.a. constant) columns `L1` and `LLAST`.

The fixed polynomials `L1` and `LLAST` are defined as `[1,0...]` and `[0...,1]` respectively. The first one is a polynomial that has a 1 in the first position and 0 in the rest of the positions. The second one is a polynomial that has a 0 in the first position and 1 in the rest of the positions.

Then, the following four constraints are defined for the AIR Fibonacci sequence:

- `(b' - a) * (1 - L1') === 0;`: This constraint defines the Fibonacci sequence unless for the last row.
- `L1 * (b - in1) === 0;`: This constraint defines that `in1` public input must be equal to the first row in the witness column `b`.
- `L1 * (a - in2) === 0;`: This constraint defines that `in2` public input must be equal to the first row in the witness column `a`.
- `LLAST * (a - out) === 0;`: This constraint defines that the last row in the witness column `a` must be equal to the public output `out`.

With this code you are constraining all the values of a Fibonacci AIR for 2<sup>10</sup> rows.

Turns out we don't define the values for the witness columns during the PIL2 design. We declare the public inputs, the fixed columns and the witness columns as well the constraints that the Fibonacci sequence must satisfy. But the fixed polynomial values will be defined because they are already known at design. The witness polynomial values that will be calculated by the witness calculator (a.k.a. executors) according to the inputs received at proof generation time.

## Generate the PILOUT file

Once the PIL2 code is ready, you can generate the PILOUT file using the [PIL2 compiler repository](https://github.com/0xPolygonHermez/pil2-compiler.git). Clone the repository and compile the PIL2 compiler by running the following commands:

```bash
node ./pil2-compiler/src/pil.js fibonacci.pil
```

This command will generate a `fibonacci.pilout` file that contains the arithmetization, public inputs, constraints, constant values, and other proof-generation-specific details described by the PIL2 project.

## Create the Fibonacci Rust project

Now that you have the `fibonacci.pilout` file, you can create a new Rust project that will generate the Fibonacci sequence. Create a new Rust project using by running the following command:

```bash
cd pil2-proofman
cargo run --bin proofman-cli new ../fibonacci --pilout ../fibonacci.pilout
```

This will create a new Rust project inside the `fibonacci` folder with the following structure:

````
├── data
│   └── fibonacci.pilout
├── src
│    ├── witness_computation
│    │   ├── fibonacci_traces.rs
│    │   ├── fibonacci_executor.rs
│    │   └── mod.rs
│    ├── main.rs
│    └── mod.rs
├── .gitignore
├── Cargo.toml
└── proofman.config.json
```

### Witness computation
Let's take a look at the `witness computation` folder. with some files inside. On the one hand it has created the `fibonacci_trace.rs` which defines a Rust struct data to manage the trace for the witness polynomials as as are defined in the PIL2 project.

Note: By the time being we are working by default with Goldilocks but we will add soon a new feature to select the finite field to be used.

```rust
use proofman_common::trace;
use goldilocks::Goldilocks;

trace!(Fibonacci {
	a: Goldilocks,
	b: Goldilocks,
});
```

On the other hand it has created the `fibonacci_executor.rs` file that specifies how the fibonacci witness calculator (or executor) fills the trace with the Fibonacci sequence.

```rust
use proofman::{executor, executor::Executor, ProofCtx, trace};
use goldilocks::{Goldilocks, AbstractField};
use log::debug;

executor!(FibonacciExecutor);

impl Executor<Goldilocks> for FibonacciExecutor {
    fn witness_computation(&self, stage_id: u32, proof_ctx: &mut ProofCtx<Goldilocks>) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        let airgroup_id = proof_ctx.pilout.find_airgroup_id_by_name("Fibonacci").expect("Airgroup not found");
        let air_id = 0;
        let num_rows = proof_ctx.pilout.air_groups[airgroup_id].airs[air_id].num_rows.unwrap() as usize;

        trace!(Fibonacci { a: Goldilocks, b: Goldilocks });
        let mut fib = Fibonacci::new(num_rows);

        fib.a[0] = Goldilocks::one();
        fib.b[0] = Goldilocks::one();

        for i in 1..num_rows {
            fib.a[i] = fib.b[i - 1];
            fib.b[i] = fib.a[i - 1] + fib.b[i - 1];
        }

        proof_ctx.add_trace_to_air_instance(airgroup_id, air_id, fib).expect("Error adding trace to air instance");
    }
}

```

The `mod.rs` file defines the module for the Fibonacci sequence. The `main.rs` file contains the main function that will generate the Fibonacci sequence. The `mod.rs` file defines the module for the Fibonacci sequence. The `Cargo.toml` file contains the dependencies for the Fibonacci sequence. The `proofman.config.json` file contains the configuration for the Fibonacci sequence.



In the `src` folder you will find the Rust code that will generate the Fibonacci sequence. The `witness_computation` folder contains the `fibonacci_traces.rs` file that defines the witness computation for the Fibonacci sequence, the `fibonacci_executor.rs` file that defines the executor for the Fibonacci sequence, and the `mod.rs` file that defines the module for the Fibonacci sequence. The `main.rs` file contains the main function that will generate the Fibonacci sequence. The `mod.rs` file defines the module for the Fibonacci sequence. The `Cargo.toml` file contains the dependencies for the Fibonacci sequence. The `proofman.config.json` file contains the configuration for the Fibonacci sequence.
