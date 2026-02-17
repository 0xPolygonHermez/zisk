use anyhow::Result;
use zisk_sdk::{ZiskStdin, ZiskIO, ElfBinary, ProofOpts, ProverClient, include_elf};

pub const ELF: ElfBinary = include_elf!("sha-hasher-guest");

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client (Compressed proof mode)...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);
    println!("Input prepared: {} iterations", n);

    // Create a `ProverClient` method.
    println!("Building prover client...");
    let client = ProverClient::builder().asm().base_port(54321).build().unwrap();

    println!("Setting up program...");
    let vkey = client.setup(&ELF)?;
    println!("Setup completed successfully");

    println!("Generating Vadcop proof...");
    let proof_opts = ProofOpts::default().minimal_memory();
    let vadcop_result = client.prove(stdin).with_proof_options(proof_opts).run()?;
    println!("Vadcop proof generated in {:?}", vadcop_result.get_duration());

    println!("Compressing proof (this may take a while)...");
    let compressed_result =
        client.compress(vadcop_result.get_proof(), vadcop_result.get_publics(), &vkey)?;

    // Alternatively, you can also call `compressed()` on the `ProverClient.prove` method to generate a compressed proof directly.
    // let result = client.prove(stdin).with_proof_options(proof_opts).compressed().run()?;

    println!("Verifying compressed proof...");
    client.verify(
        compressed_result.get_proof(),
        compressed_result.get_publics(),
        compressed_result.get_program_vk(),
    )?;
    println!("Compressed proof verification successful!");

    println!("\u{2713} Successfully generated and verified compressed proof!");

    Ok(())
}
