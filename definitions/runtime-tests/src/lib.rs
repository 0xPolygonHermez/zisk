//! Differential tests: the no_std Rust runtime, the C++ ASM helper and the hint handler
//! against the pinned reference, plus rejection of every noncanonical lane.

#[path = "../../src/koala_poseidon2.rs"]
pub mod runtime;

#[path = "../../../ziskos-hints/src/handlers/koala_poseidon2.rs"]
pub mod hints;

#[cfg(test)]
mod tests {
    use super::runtime;
    use zisk_koala_poseidon2_foundation::{Parameters, State, MODULUS};

    extern "C" {
        fn koala_native_test(state: *mut u64) -> bool;
    }

    fn packed(state: State) -> [u64; 8] {
        core::array::from_fn(|i| u64::from(state[2 * i]) | u64::from(state[2 * i + 1]) << 32)
    }

    #[test]
    fn rust_and_cpp_match_pinned_reference() {
        let reference = Parameters::pinned();
        let mut cases = vec![[0; 16], [1; 16], [MODULUS - 1; 16]];
        for lane in 0..16 {
            for value in [1, MODULUS - 1] {
                let mut state = [0; 16];
                state[lane] = value;
                cases.push(state);
            }
        }
        let mut seed = 0x3948_a0fd_1279_eb12_u64;
        for _ in 0..128 {
            cases.push(core::array::from_fn(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ((seed >> 32) % u64::from(MODULUS)) as u32
            }));
        }
        for input in cases {
            let expected = packed(reference.permute(input).unwrap());
            let mut rust = packed(input);
            assert_eq!(runtime::validate_packed(&rust), Ok(()));
            runtime::permute_packed(&mut rust).unwrap();
            assert_eq!(rust, expected);
            let mut cpp = packed(input);
            assert!(unsafe { koala_native_test(cpp.as_mut_ptr()) });
            assert_eq!(cpp, expected);
        }
    }

    #[test]
    fn all_noncanonical_lanes_reject_without_mutating_input() {
        for lane in 0..16 {
            for value in [MODULUS, MODULUS + 1, 1 << 31, u32::MAX] {
                let mut state = [0; 16];
                state[lane] = value;
                let input = packed(state);
                assert_eq!(runtime::validate_packed(&input), Err(runtime::NoncanonicalInput(lane)));
                let mut rust = input;
                assert_eq!(
                    runtime::permute_packed(&mut rust),
                    Err(runtime::NoncanonicalInput(lane))
                );
                assert_eq!(rust, input);
                let mut cpp = input;
                assert!(!unsafe { koala_native_test(cpp.as_mut_ptr()) });
                assert_eq!(cpp, input);
            }
        }
    }

    #[test]
    fn hints_match_reference_and_reject_bad_encodings() {
        let input = packed(core::array::from_fn(|i| (i as u32 * 13) % MODULUS));
        let mut expected = input;
        runtime::permute_packed(&mut expected).unwrap();
        assert_eq!(super::hints::koala_poseidon2_hint(&input, 64).unwrap(), expected);
        for length in [0, 1, 63, 65, 128] {
            assert!(super::hints::koala_poseidon2_hint(&input, length).is_err());
        }
        assert!(super::hints::koala_poseidon2_hint(&input[..7], 64).is_err());
        assert!(super::hints::koala_poseidon2_hint(&[0; 9], 64).is_err());
        for lane in 0..16 {
            let mut state = [0; 16];
            state[lane] = MODULUS;
            assert!(super::hints::koala_poseidon2_hint(&packed(state), 64).is_err());
        }
    }
}
