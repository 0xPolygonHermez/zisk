use zisk_sdk::{load_program, GuestProgram};

pub const ELF_SHA_HASHER: GuestProgram = load_program!("sha_hasher");
// pub const ELF_SHA_HASHER: GuestProgram =
//     load_program!("sha_hasher", "../guest/target/elf/riscv64ima-zisk-zkvm-elf/release/sha_hasher");
