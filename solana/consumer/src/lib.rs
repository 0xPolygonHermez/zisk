#![allow(unexpected_cfgs)]
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};
mod config;
solana_program::entrypoint!(process_instruction);

// A signed receipt stores the output digest; repeating a proof is idempotent.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    outputs: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 || outputs.len() != 512 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let receipt = &accounts[0];
    let buffer = &accounts[1];
    let verifier = &accounts[2];
    if verifier.key != &config::VERIFIER_ID || !verifier.executable {
        return Err(ProgramError::IncorrectProgramId);
    }
    if receipt.owner != program_id || receipt.data_len() != 32 || !receipt.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }
    if !receipt.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let mut data = Vec::with_capacity(545);
    data.push(3);
    data.extend_from_slice(&config::GUEST_KEY);
    data.extend_from_slice(outputs);
    invoke(
        &Instruction {
            program_id: config::VERIFIER_ID,
            accounts: vec![AccountMeta::new_readonly(*buffer.key, false)],
            data,
        },
        &[buffer.clone(), verifier.clone()],
    )?;
    receipt.try_borrow_mut_data()?.copy_from_slice(&solana_program::hash::hash(outputs).to_bytes());
    Ok(())
}
