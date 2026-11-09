#![allow(unexpected_cfgs)]
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use solana_sdk_ids::{bpf_loader_upgradeable, system_program};
use solana_system_interface::instruction as system_instruction;
mod config;
solana_program::entrypoint!(process_instruction);

// The permanent PDA is a per-statement replay guard, including across different uploads.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction: &[u8],
) -> ProgramResult {
    if accounts.len() != 6 || instruction.len() != 513 || instruction[0] > 1 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let outputs = &instruction[1..];
    let payer = &accounts[0];
    let receipt = &accounts[1];
    let buffer = &accounts[2];
    let verifier = &accounts[3];
    let programdata = &accounts[4];
    let system = &accounts[5];
    if verifier.key != &config::VERIFIER_ID || !verifier.executable {
        return Err(ProgramError::IncorrectProgramId);
    }
    if verifier.owner != &bpf_loader_upgradeable::ID
        || programdata.owner != &bpf_loader_upgradeable::ID
    {
        return Err(ProgramError::IllegalOwner);
    }
    let program_state: UpgradeableLoaderState = bincode::deserialize(&verifier.try_borrow_data()?)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    if !matches!(program_state, UpgradeableLoaderState::Program { programdata_address } if programdata_address == *programdata.key)
    {
        return Err(ProgramError::InvalidAccountData);
    }
    let deployment: UpgradeableLoaderState = bincode::deserialize(&programdata.try_borrow_data()?)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    if !matches!(
        deployment,
        UpgradeableLoaderState::ProgramData { upgrade_authority_address: None, .. }
    ) {
        return Err(ProgramError::InvalidAccountData);
    }
    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !payer.is_writable
        || !receipt.is_writable
        || system.key != &system_program::ID
        || buffer.owner != verifier.key
    {
        return Err(ProgramError::InvalidAccountData);
    }
    let digest = zisk_solana_program::statement_digest(&config::GUEST_KEY, outputs);
    let (expected, bump) = Pubkey::find_program_address(&[b"consumed", &digest], program_id);
    if receipt.key != &expected {
        return Err(ProgramError::InvalidSeeds);
    }
    if receipt.owner != &system_program::ID || !receipt.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    if instruction[0] == 1 {
        if zisk_solana_program::verified_statement(&buffer.try_borrow_data()?)? != digest {
            return Err(ProgramError::InvalidAccountData);
        }
    } else {
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
    }
    // Allocate/assign also handles unsolicited lamports sent to this predictable PDA.
    let top_up = Rent::get()?.minimum_balance(32).saturating_sub(receipt.lamports());
    if top_up != 0 {
        invoke(
            &system_instruction::transfer(payer.key, receipt.key, top_up),
            &[payer.clone(), receipt.clone(), system.clone()],
        )?;
    }
    let bump = [bump];
    let seeds: &[&[u8]] = &[b"consumed", &digest, &bump];
    invoke_signed(
        &system_instruction::allocate(receipt.key, 32),
        &[receipt.clone(), system.clone()],
        &[seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(receipt.key, program_id),
        &[receipt.clone(), system.clone()],
        &[seeds],
    )?;
    receipt.try_borrow_mut_data()?.copy_from_slice(&digest);
    Ok(())
}
