#![allow(unexpected_cfgs)]
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};
use zisk_solana_verifier::{PROOF_BYTES, PUBLIC_BYTES};
#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

pub const HEADER_BYTES: usize = 45;
pub const BUFFER_BYTES: usize = HEADER_BYTES + PROOF_BYTES;
pub const CHUNK_BYTES: usize = 900;
const MAGIC: &[u8; 8] = b"ZKPF0001";

fn header(data: &[u8]) -> Result<(usize, bool), ProgramError> {
    if &data[..8] != MAGIC || data[44] > 1 {
        return Err(ProgramError::InvalidAccountData);
    }
    let written = u32::from_le_bytes(data[40..44].try_into().unwrap()) as usize;
    let sealed = data[44] == 1;
    if written > PROOF_BYTES || (sealed && written != PROOF_BYTES) {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok((written, sealed))
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction: &[u8],
) -> ProgramResult {
    let op = *instruction.first().ok_or(ProgramError::InvalidInstructionData)?;
    if op > 4 || accounts.len() != if op == 3 { 1 } else { 2 } {
        return Err(ProgramError::InvalidInstructionData);
    }
    let buffer = &accounts[0];
    if buffer.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }
    if buffer.data_len() != BUFFER_BYTES {
        return Err(ProgramError::InvalidAccountData);
    }
    if op == 3 {
        if instruction.len() != 1 + 32 + PUBLIC_BYTES {
            return Err(ProgramError::InvalidInstructionData);
        }
        let data = buffer.try_borrow_data()?;
        if !header(&data)?.1 {
            return Err(ProgramError::InvalidAccountData);
        }
        return zisk_solana_verifier::verify(
            instruction[1..33].try_into().unwrap(),
            &instruction[33..],
            &data[HEADER_BYTES..],
        )
        .map_err(|e| ProgramError::Custom(e as u32));
    }
    let authority = &accounts[1];
    if !buffer.is_writable || buffer.key == authority.key {
        return Err(ProgramError::InvalidAccountData);
    }
    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let mut data = buffer.try_borrow_mut_data()?;
    if op == 0 {
        if instruction.len() != 5
            || u32::from_le_bytes(instruction[1..5].try_into().unwrap()) as usize != PROOF_BYTES
        {
            return Err(ProgramError::InvalidInstructionData);
        }
        if !buffer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if data.iter().any(|b| *b != 0) {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        data[..8].copy_from_slice(MAGIC);
        data[8..40].copy_from_slice(authority.key.as_ref());
        return Ok(());
    }
    let (written, sealed) = header(&data)?;
    if &data[8..40] != authority.key.as_ref() {
        return Err(ProgramError::IllegalOwner);
    }
    match op {
        1 => {
            if instruction.len() < 6 || instruction.len() > 5 + CHUNK_BYTES || sealed {
                return Err(ProgramError::InvalidInstructionData);
            }
            let offset = u32::from_le_bytes(instruction[1..5].try_into().unwrap()) as usize;
            let end = offset
                .checked_add(instruction.len() - 5)
                .ok_or(ProgramError::InvalidInstructionData)?;
            if offset != written || end > PROOF_BYTES {
                return Err(ProgramError::InvalidInstructionData);
            }
            data[HEADER_BYTES + offset..HEADER_BYTES + end].copy_from_slice(&instruction[5..]);
            data[40..44].copy_from_slice(&(end as u32).to_le_bytes());
        }
        2 => {
            if instruction.len() != 1 || sealed || written != PROOF_BYTES {
                return Err(ProgramError::InvalidInstructionData);
            }
            data[44] = 1;
        }
        4 => {
            if instruction.len() != 1 || !authority.is_writable {
                return Err(ProgramError::InvalidInstructionData);
            }
            let balance = authority
                .lamports()
                .checked_add(buffer.lamports())
                .ok_or(ProgramError::ArithmeticOverflow)?;
            data.fill(0);
            **authority.try_borrow_mut_lamports()? = balance;
            **buffer.try_borrow_mut_lamports()? = 0;
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    }
    Ok(())
}
