use zisk_sdk::{load_program, GuestProgram};

pub const ELF_SHA_HASHER: GuestProgram = load_program!("sha_hasher");
