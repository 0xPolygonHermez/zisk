//! ZisK host for the MLP example: builds a deterministic integer neural
//! network, runs one inference natively, executes and proves the same
//! inference in ZisK, and reports the cost.
//!
//! # Usage
//!
//! ```text
//! cargo run --release -- [WIDTH...] [--execute-only] [--save-input PATH] [--asm] [--gpu]
//! ```
//!
//! `WIDTH...` are the layer widths, input first and output last. The default
//! is `784 128 10`, an MNIST-sized network. For example `784 512 512 10`
//! adds a second, wider hidden layer.
//!
//! - `--execute-only` counts steps without proving, to scan sizes quickly.
//! - `--save-input PATH` writes the guest input file for `cargo-zisk` and exits.

use std::error::Error;
use std::time::Instant;

use mlp_common::{DEFAULT_SEED, Model, argmax, generate_input, generate_model, infer};
use zisk_sdk::{GuestProgram, ProverClient, ZiskStdin, load_program};

/// Guest ELF binary, embedded into the host at build time.
static PROGRAM: GuestProgram = load_program!("mlp-guest");

struct Args {
    dims: Vec<u32>,
    execute_only: bool,
    save_input: Option<String>,
    asm: bool,
    gpu: bool,
}

fn parse_args() -> Result<Args, Box<dyn Error>> {
    let mut args = Args {
        dims: Vec::new(),
        execute_only: false,
        save_input: None,
        asm: false,
        gpu: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--execute-only" => args.execute_only = true,
            "--save-input" => args.save_input = Some(it.next().ok_or("--save-input needs a path")?),
            "--asm" => args.asm = true,
            "--gpu" => args.gpu = true,
            width => args.dims.push(width.parse().map_err(|_| format!("bad width: {width}"))?),
        }
    }
    if args.dims.is_empty() {
        args.dims = vec![784, 128, 10];
    }
    if args.dims.len() < 2 || args.dims.contains(&0) {
        return Err("give at least two non-zero widths (input and output)".into());
    }
    Ok(args)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;

    // Deterministic model and input: same widths, same bytes, every run.
    let blob = generate_model(&args.dims, DEFAULT_SEED);
    let input = generate_input(args.dims[0] as usize, DEFAULT_SEED);
    let model = Model::parse(&blob);

    let input_bytes: Vec<u8> = input.iter().map(|&v| v as u8).collect();
    let stdin = ZiskStdin::new();
    stdin.write_slice(&blob);
    stdin.write_slice(&input_bytes);

    if let Some(path) = &args.save_input {
        stdin.save(path)?;
        println!("Saved guest input for {:?} to {path}", args.dims);
        return Ok(());
    }

    // Reference result, computed natively with the same code the guest runs.
    let expected_logits = infer(&model, &input);
    let expected_class = argmax(&expected_logits);

    println!("Model {:?}", args.dims);
    println!("  parameters: {}", model.params());
    println!("  MACs/inference: {}", model.macs());
    println!("  model blob: {} bytes", blob.len());

    let mut builder = ProverClient::embedded();
    if args.asm {
        builder = builder.assembly();
    }
    if args.gpu {
        builder = builder.gpu();
    }
    let client = builder.build()?;

    let t = Instant::now();
    client.setup(&PROGRAM).run_sync()?;
    println!("Setup: {:.2?}", t.elapsed());

    let result = client.execute(&PROGRAM, &stdin).run_sync()?;
    let steps = result.get_execution_steps();
    println!(
        "Execute: {steps} steps in {} ms ({:.1} steps/MAC)",
        result.get_execution_time(),
        steps as f64 / model.macs() as f64
    );

    if args.execute_only {
        return Ok(());
    }

    let t = Instant::now();
    let proof = client.prove(&PROGRAM, &stdin).run_sync()?;
    println!("Prove: {:.2?}", t.elapsed());

    let t = Instant::now();
    proof.with_program_vk(&PROGRAM.vk()?).verify()?;
    println!("Verify: {:.2?}", t.elapsed());

    // The committed public values must match the native run.
    let (class, logits) = proof.get_publics().read::<(u32, Vec<i32>)>()?;
    assert_eq!(logits, expected_logits, "proven logits differ from the native run");
    assert_eq!(class, expected_class);
    println!("Proven class {class}, matches the native run");

    Ok(())
}
