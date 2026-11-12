#![no_main]
use libfuzzer_sys::fuzz_target;
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};
use zisk_solana_program::{process_instruction, BUFFER_BYTES, HEADER_BYTES};

fuzz_target!(|input: &[u8]| {
    let program = Pubkey::new_from_array([41; 32]);
    let key = Pubkey::new_from_array([42; 32]);
    let actors = [Pubkey::new_from_array([43; 32]), Pubkey::new_from_array([44; 32])];
    let system = Pubkey::default();
    let mut bytes = vec![0; BUFFER_BYTES];
    let mut balance = 100_000u64;
    let mut funds = [100_000u64; 2];
    let mut owner: Option<usize> = None;
    let mut written = 0usize;
    let mut sealed = false;
    for event in input.chunks_exact(6).take(64) {
        let alias = event[0] % 7 == 6;
        let op = if alias { 4 } else { event[0] % 7 };
        let flags = event[1];
        let actor = usize::from((flags >> 4) & 1);
        let signed = flags & 1 == 0;
        let writable = flags & 2 == 0;
        let key_signed = flags & 4 == 0;
        let actor_writable = flags & 8 == 0;
        let length = match (flags >> 5) & 3 {
            0 => BUFFER_BYTES,
            1 => BUFFER_BYTES - 1,
            2 => BUFFER_BYTES + 1,
            _ => 0,
        };
        let correct_owner = flags & 128 == 0;
        let offset = match event[2] % 5 {
            0 => written,
            1 => 0,
            2 => written + 1,
            3 => written.saturating_sub(1),
            _ => u32::MAX as usize,
        };
        let count = match event[3] % 5 {
            0 => 900,
            1 => 6752 - written,
            2 => 1,
            3 => 0,
            _ => 901,
        };
        let exact = event[5] & 1 == 0;
        let mut instruction = vec![op];
        match op {
            0 => instruction.extend_from_slice(&(if exact { 6752u32 } else { 6751 }).to_le_bytes()),
            1 => {
                instruction.extend_from_slice(&(offset as u32).to_le_bytes());
                instruction.resize(5 + count, event[4]);
            }
            3 | 5 => instruction.resize(if exact { 545 } else { 544 }, 0),
            _ => {
                if !exact {
                    instruction.push(0);
                }
            }
        }
        let mut data = bytes.clone();
        data.resize(length, 0);
        let snapshot = data.clone();
        let before = balance;
        let actor_before = funds[actor];
        let expected = correct_owner
            && length == BUFFER_BYTES
            && !alias
            && match op {
                0 => writable && signed && key_signed && owner.is_none() && exact,
                1 => {
                    writable
                        && signed
                        && owner == Some(actor)
                        && !sealed
                        && count > 0
                        && count <= 900
                        && offset == written
                        && offset + count <= 6752
                }
                2 => {
                    writable
                        && signed
                        && owner == Some(actor)
                        && !sealed
                        && written == 6752
                        && exact
                }
                4 => writable && signed && actor_writable && owner == Some(actor) && exact,
                _ => false,
            };
        let result = {
            let buffer = AccountInfo::new(
                &key,
                key_signed,
                writable,
                &mut balance,
                &mut data,
                if correct_owner { &program } else { &system },
                false,
                0,
            );
            let mut empty = [];
            let authority = AccountInfo::new(
                &actors[actor],
                signed,
                actor_writable,
                &mut funds[actor],
                &mut empty,
                &system,
                false,
                0,
            );
            let accounts = if alias {
                vec![buffer.clone(), buffer]
            } else if op == 3 || op == 5 {
                vec![buffer]
            } else {
                vec![buffer, authority]
            };
            process_instruction(&program, &accounts, &instruction)
        };
        if op == 3 || op == 5 {
            // Synthetic record attempts use invalid proofs; failures must not mutate state.
            if op == 5 || !correct_owner || length != BUFFER_BYTES || !sealed || !exact {
                assert!(result.is_err());
            }
            assert_eq!(data, snapshot);
            assert_eq!(balance, before);
            assert_eq!(funds[actor], actor_before);
            continue;
        }
        assert_eq!(
            result.is_ok(),
            expected,
            "op={op} flags={flags} written={written} sealed={sealed}"
        );
        if !expected {
            assert_eq!(data, snapshot);
            assert_eq!(balance, before);
            assert_eq!(funds[actor], actor_before);
            continue;
        }
        let mut expected_data = bytes.clone();
        match op {
            0 => {
                expected_data[..8].copy_from_slice(b"ZKPF0002");
                expected_data[8..40].copy_from_slice(actors[actor].as_ref());
                owner = Some(actor);
            }
            1 => {
                expected_data[HEADER_BYTES + offset..HEADER_BYTES + offset + count].fill(event[4]);
                written += count;
                expected_data[40..44].copy_from_slice(&(written as u32).to_le_bytes());
            }
            2 => {
                expected_data[44] = 1;
                sealed = true;
            }
            4 => {
                expected_data.fill(0);
                owner = None;
                written = 0;
                sealed = false;
            }
            _ => unreachable!(),
        }
        assert_eq!(data, expected_data);
        if op == 4 {
            assert_eq!(balance, 0);
            assert_eq!(funds[actor], actor_before + before);
        } else {
            assert_eq!(balance, before);
            assert_eq!(funds[actor], actor_before);
        }
        bytes = data;
    }
});
