use anyhow::Result;
use zisk_sdk::{
    include_guest_elf, EmbeddedGuestElf, GuestProgram, ProofOpts, ProverClient, ZiskStdin,
};

pub const ELF: EmbeddedGuestElf = include_guest_elf!("sha-hasher-guest");

fn main() -> Result<()> {
    println!("Starting ZisK Prover Client...");

    // Create an input stream and write '1000' to it.
    let n = 1000u32;
    let stdin = ZiskStdin::new();
    stdin.write(&n);

    // Create a `ProverClient` method.
    let client = ProverClient::builder().asm().build().unwrap();

    let (pk, vkey) = client.setup(&GuestProgram::from_elf(ELF))?;

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let result = client.execute(&pk, stdin.clone())?;

    println!(
        "ZisK has executed program with {} cycles in {:?}",
        result.get_execution_steps(),
        result.get_duration()
    );

    let proof_opts = ProofOpts::default().minimal_memory();
    let vadcop_result = client.prove(&pk, stdin).with_proof_options(proof_opts).run()?;
    vadcop_result.program_vk(&vkey).verify()?;

    println!("successfully generated and verified proof for the program!");

    Ok(())
}
