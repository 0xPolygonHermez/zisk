mod gate;
mod gate_config;
mod gate_state;
mod pin;
mod utils;

pub use gate::{Gate, GateOperation};
pub use gate_config::GateConfig;
pub use gate_state::GateState;
pub use pin::{Pin, PinId, PinSource};
pub use utils::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_gate_circuits() {
        #[rustfmt::skip]
        let circuit_config = GateConfig::with_values(
            4,
            4,
            None,
            0,
            1,
            2,
            1,
            2,
            1,
            1,
            1,
        );

        let mut state = GateState::new(circuit_config.clone());

        //      c
        //      |
        //     (^)
        //    /   \
        //   a     b
        let inputs = [[0, 0], [0, 1], [1, 0], [1, 1]];
        let expected_outputs = [0, 1, 1, 0];
        for (i, input) in inputs.iter().enumerate() {
            // Copy input bits to the state
            for (j, &bit) in input.iter().enumerate() {
                let ref_idx =
                    circuit_config.sin_first_ref + j as u64 * circuit_config.sin_ref_distance;
                state.gates[ref_idx as usize].pins[PinId::A].bit = bit;
            }

            // Perform the circuit operation
            let free_ref = state.get_free_ref();
            state.xor(state.sin_refs[0], PinId::A, state.sin_refs[1], PinId::A, free_ref);

            // Copy the result to output
            let output_ref = circuit_config.sout_first_ref;
            state.gates[output_ref as usize].pins[PinId::A].bit =
                state.gates[free_ref as usize].pins[PinId::D].bit;

            // Compare
            assert_eq!(state.gates[output_ref as usize].pins[PinId::A].bit, expected_outputs[i]);

            // Reset the circuit for the next iteration
            state.reset_bits_and_counters();
        }

        //      c
        //      |
        //    (¬ ^)
        //    /   \
        //   a     b
        let inputs = [[0, 0], [0, 1], [1, 0], [1, 1]];
        let expected_outputs = [0, 1, 0, 0];
        for (i, input) in inputs.iter().enumerate() {
            // Copy input bits to the state
            for (j, &bit) in input.iter().enumerate() {
                let ref_idx =
                    circuit_config.sin_first_ref + j as u64 * circuit_config.sin_ref_distance;
                state.gates[ref_idx as usize].pins[PinId::A].bit = bit;
            }

            // Perform the circuit operation
            let free_ref = state.get_free_ref();
            state.andp(state.sin_refs[0], PinId::A, state.sin_refs[1], PinId::A, free_ref);

            // Copy the result to output
            let output_ref = circuit_config.sout_first_ref;
            state.gates[output_ref as usize].pins[PinId::A].bit =
                state.gates[free_ref as usize].pins[PinId::D].bit;

            // Compare
            assert_eq!(state.gates[output_ref as usize].pins[PinId::A].bit, expected_outputs[i]);

            // Reset the circuit for the next iteration
            state.reset_bits_and_counters();
        }

        //      c
        //      |
        //     (|)
        //    /   \
        //   a     b
        let inputs = [[0, 0], [0, 1], [1, 0], [1, 1]];
        let expected_outputs = [0, 1, 1, 1];
        for (i, input) in inputs.iter().enumerate() {
            // Copy input bits to the state
            for (j, &bit) in input.iter().enumerate() {
                let ref_idx =
                    circuit_config.sin_first_ref + j as u64 * circuit_config.sin_ref_distance;
                state.gates[ref_idx as usize].pins[PinId::A].bit = bit;
            }

            // Perform the circuit operation
            let free_ref = state.get_free_ref();
            state.or(state.sin_refs[0], PinId::A, state.sin_refs[1], PinId::A, free_ref);

            // Copy the result to output
            let output_ref = circuit_config.sout_first_ref;
            state.gates[output_ref as usize].pins[PinId::A].bit =
                state.gates[free_ref as usize].pins[PinId::D].bit;

            // Compare
            assert_eq!(state.gates[output_ref as usize].pins[PinId::A].bit, expected_outputs[i]);

            // Reset the circuit for the next iteration
            state.reset_bits_and_counters();
        }

        //      c
        //      |
        //     (&)
        //    /   \
        //   a     b
        let inputs = [[0, 0], [0, 1], [1, 0], [1, 1]];
        let expected_outputs = [0, 0, 0, 1];
        for (i, input) in inputs.iter().enumerate() {
            // Copy input bits to the state
            for (j, &bit) in input.iter().enumerate() {
                let ref_idx =
                    circuit_config.sin_first_ref + j as u64 * circuit_config.sin_ref_distance;
                state.gates[ref_idx as usize].pins[PinId::A].bit = bit;
            }

            // Perform the circuit operation
            let free_ref = state.get_free_ref();
            state.and(state.sin_refs[0], PinId::A, state.sin_refs[1], PinId::A, free_ref);

            // Copy the result to output
            let output_ref = circuit_config.sout_first_ref;
            state.gates[output_ref as usize].pins[PinId::A].bit =
                state.gates[free_ref as usize].pins[PinId::D].bit;

            // Compare
            assert_eq!(state.gates[output_ref as usize].pins[PinId::A].bit, expected_outputs[i]);

            // Reset the circuit for the next iteration
            state.reset_bits_and_counters();
        }
    }

    #[test]
    fn test_simple_circuit() {
        //         c
        //         |
        //        (^)
        //       /   \
        //      /     \
        //     (&)     b
        //    /   \
        //   1     a
        #[rustfmt::skip]
        let circuit_config = GateConfig::with_values(
            6,
            6,
            Some(0),
            1,
            1,
            2,
            1,
            3,
            1,
            1,
            1,
        );

        let mut state = GateState::new(circuit_config.clone());
        let inputs = [[0, 0], [1, 1], [0, 1], [1, 0]];
        let expected_outputs = [0, 0, 1, 1];
        for (i, input) in inputs.iter().enumerate() {
            // Copy input bits to the state
            for (j, &bit) in input.iter().enumerate() {
                let ref_idx =
                    circuit_config.sin_first_ref + j as u64 * circuit_config.sin_ref_distance;
                state.gates[ref_idx as usize].pins[PinId::A].bit = bit;
            }

            // The ZeroRef is by default XOR(0,1) = 1, so we dont need to modify it

            // Perform the circuit operation
            // 1] res := AND(1, a)
            let free_ref1 = state.get_free_ref();
            state.and(
                state.gate_config.zero_ref.unwrap(),
                PinId::B,
                state.sin_refs[0],
                PinId::A,
                free_ref1,
            );

            // 2] XOR(res, b)
            let free_ref2 = state.get_free_ref();
            state.xor(free_ref1, PinId::D, state.sin_refs[1], PinId::A, free_ref2);

            // Copy the result to output
            let output_ref = circuit_config.sout_first_ref;
            state.gates[output_ref as usize].pins[PinId::A].bit =
                state.gates[free_ref2 as usize].pins[PinId::D].bit;

            // Compare
            assert_eq!(state.gates[output_ref as usize].pins[PinId::A].bit, expected_outputs[i]);

            // Reset the circuit for the next iteration
            state.reset_bits_and_counters();
        }
    }
}
