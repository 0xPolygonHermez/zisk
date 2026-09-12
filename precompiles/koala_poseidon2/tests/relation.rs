use slop_algebra::{AbstractField, PrimeField32};
use slop_koala_bear::{my_kb_16_perm, DiffusionMatrixKoalaBear, KoalaBear, MONTY_INVERSE};
use slop_poseidon2::Poseidon2ExternalMatrixGeneral;
use slop_symmetric::Permutation;
use zisk_koala_poseidon2_foundation::{
    decode_input, encode_output, rounds::Rounds, Parameters, State, MODULUS, WIDTH,
};

fn upstream(input: State) -> State {
    let mut state = input.map(KoalaBear::from_canonical_u32);
    my_kb_16_perm().permute_mut(&mut state);
    state.map(|x| x.as_canonical_u32())
}

#[test]
fn exact_pinned_permutation_matches_boundaries_basis_and_random_states() {
    let parameters = Parameters::pinned();
    let rounds = Rounds::pinned();
    let mut cases = vec![[0; WIDTH], [1; WIDTH], [MODULUS - 1; WIDTH]];
    for lane in 0..WIDTH {
        for value in [1, MODULUS - 1] {
            let mut input = [0; WIDTH];
            input[lane] = value;
            cases.push(input);
        }
    }
    let mut seed = 0x729c_9a17_527e_d210_u64;
    for _ in 0..128 {
        cases.push(std::array::from_fn(|_| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 32) % u64::from(MODULUS)) as u32
        }));
    }
    for input in cases {
        let expected = upstream(input);
        assert_eq!(parameters.permute(input).unwrap(), expected);
        assert_eq!(rounds.execute(input, |_, _| {}).unwrap(), expected);
    }
}

#[test]
fn extracted_matrices_preserve_orientation_and_montgomery_scaling() {
    let parameters = Parameters::pinned();
    for lane in 0..WIDTH {
        let mut basis = [KoalaBear::zero(); WIDTH];
        basis[lane] = KoalaBear::one();
        let mut external = basis;
        Poseidon2ExternalMatrixGeneral.permute_mut(&mut external);
        let mut internal = basis;
        DiffusionMatrixKoalaBear.permute_mut(&mut internal);
        for i in 0..WIDTH {
            assert_eq!(parameters.external[i][lane], external[i].as_canonical_u32());
            assert_eq!(parameters.internal[i][lane], internal[i].as_canonical_u32());
            if i != lane {
                assert_eq!(parameters.internal[i][lane], MONTY_INVERSE.as_canonical_u32());
            }
        }
    }
    assert_ne!(parameters.external[0][1], parameters.external[1][0]);
    assert_ne!(MONTY_INVERSE.as_canonical_u32(), 1);
    let mut wrong = parameters.clone();
    let scale = KoalaBear::from_canonical_u64((1_u64 << 32) % u64::from(MODULUS));
    for row in &mut wrong.internal {
        for coefficient in row {
            *coefficient = (KoalaBear::from_canonical_u32(*coefficient) * scale).as_canonical_u32();
        }
    }
    assert_ne!(wrong.permute([1; WIDTH]).unwrap(), upstream([1; WIDTH]));
}

#[test]
fn abi_is_canonical_little_endian_and_rejects_every_noncanonical_lane() {
    let state = std::array::from_fn(|i| if i % 2 == 0 { i as u32 } else { MODULUS - 1 });
    let bytes = encode_output(state).unwrap();
    assert_eq!(decode_input(&bytes).unwrap(), state);
    assert_eq!(&bytes[4..8], &(MODULUS - 1).to_le_bytes());
    assert!(decode_input(&bytes[..63]).is_err());
    assert!(decode_input(&[0; 65]).is_err());
    let parameters = Parameters::pinned();
    let rounds = Rounds::pinned();
    for lane in 0..WIDTH {
        for invalid in [MODULUS, MODULUS + 1, 1 << 31, u32::MAX] {
            let mut state = [0; WIDTH];
            state[lane] = invalid;
            assert!(parameters.permute(state).is_err());
            assert!(rounds.execute(state, |_, _| {}).is_err());
            assert!(encode_output(state).is_err());
            let mut bytes = [0; 64];
            bytes[lane * 4..lane * 4 + 4].copy_from_slice(&invalid.to_le_bytes());
            assert!(decode_input(&bytes).is_err());
        }
    }
}

#[test]
fn vm_pil_reproduces_checked_in_constraints() {
    let source = zisk_koala_poseidon2_foundation::rounds::vm_pil();
    assert_eq!(source, include_str!("../../koala_poseidon2_vm/pil/koala_poseidon2.pil"));
    assert!(!source.contains("stage_identity"));
    assert!(!source.contains("public input"));
    assert!(source.contains("proves_operation(op: OP_KOALA_POSEIDON2"));
    assert!(source.contains("(1 - last) * (main_step' - main_step)"));
    assert!(source.contains("sel: first * in_use"));
    assert!(source.contains("sel: last * in_use"));
}
