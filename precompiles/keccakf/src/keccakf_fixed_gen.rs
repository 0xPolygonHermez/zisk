use std::error::Error;

use clap::{Arg, Command};
use fields::{Field, Goldilocks, PrimeField64};

use zisk_pil::KeccakfTrace;

use proofman_common::{write_fixed_cols_bin, FixedColsInfo};

use circuit::GateOperation;
use precompiles_common::{get_ks, log2, GOLDILOCKS_GEN, GOLDILOCKS_K};
use precompiles_helpers::keccakf_topology;

type F = Goldilocks;

type FixedCols = (Vec<F>, Vec<F>, Vec<F>, Vec<F>, Vec<F>);

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("keccakf_fixed_gen")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("output_path")
                .help("Path to the output binary file")
                .default_value("precompiles/keccakf/src/keccakf_fixed.bin"),
        )
        .get_matches();

    let output_file = matches.get_one::<String>("output").unwrap();

    let n: usize = KeccakfTrace::<Goldilocks>::NUM_ROWS;
    let bits = log2(n);

    // Get the subgroup generator and coset generator
    let subgroup_gen = GOLDILOCKS_GEN[bits];
    let cosets_gen = GOLDILOCKS_K;

    // Generate the columns
    let (conn_a, conn_b, conn_c, conn_d, gate_op) = cols_gen(n, subgroup_gen, cosets_gen);

    // Serialize the columns and write them to a binary file
    let conn_a = FixedColsInfo::new("Keccakf.CONN_A", None, conn_a);
    let conn_b = FixedColsInfo::new("Keccakf.CONN_B", None, conn_b);
    let conn_c = FixedColsInfo::new("Keccakf.CONN_C", None, conn_c);
    let conn_d = FixedColsInfo::new("Keccakf.CONN_D", None, conn_d);
    let gate_op = FixedColsInfo::new("Keccakf.GATE_OP", None, gate_op);

    write_fixed_cols_bin(
        output_file,
        "Zisk",
        "Keccakf",
        n as u64,
        &mut [conn_a, conn_b, conn_c, conn_d, gate_op],
    );
    println!("CONN_A, CONN_B, CONN_C and GATE_OP columns written to {output_file}");

    Ok(())
}

fn cols_gen(subgroup_order: usize, subgroup_gen: u64, cosets_gen: u64) -> FixedCols {
    fn connect(c1: &mut [F], i1: usize, c2: Option<&mut [F]>, i2: usize) {
        match c2 {
            Some(c2) => std::mem::swap(&mut c1[i1], &mut c2[i2]),
            None => c1.swap(i1, i2),
        }
    }

    // Get the program and gates
    let keccakf_top = keccakf_topology();
    let keccakf_program = keccakf_top.program;
    let keccakf_gates = keccakf_top.gates;

    // Check that the subgroup order is sufficiently large
    let circuit_size = keccakf_program.len();
    if circuit_size >= subgroup_order {
        panic!("The provided number of bits {subgroup_order} is too small for the Keccakf circuit");
    }

    // Get the number of circuits we can generate
    let num_circuits = (subgroup_order - 1) / circuit_size;

    // Get the coset generators "ks" and the generator "w"
    let w = F::from_u64(subgroup_gen);
    let k = F::from_u64(cosets_gen);
    let ks = get_ks(k, 3);

    // Initialize the connections with the row identifiers
    let mut wi = F::ONE;
    let mut conn_a = vec![F::ONE; subgroup_order];
    let mut conn_b = vec![F::ONE; subgroup_order];
    let mut conn_c = vec![F::ONE; subgroup_order];
    let mut conn_d = vec![F::ONE; subgroup_order];
    for i in 0..subgroup_order {
        conn_a[i] = wi;
        conn_b[i] = wi * ks[0];
        conn_c[i] = wi * ks[1];
        conn_d[i] = wi * ks[2];
        wi *= w;
    }

    // Initialize the gate_op
    let mut gate_op = vec![F::ZERO; subgroup_order];

    // Compute the connections and gate_op
    for i in 0..num_circuits {
        let offset = i * circuit_size;

        // Compute the connections. The "+1" is for the zero_ref gate
        for (j, gate) in keccakf_gates.iter().enumerate() {
            let mut ref1 = j;
            if j > 0 {
                ref1 += offset;
            }

            // k = 0: Connections to input A
            // k = 1: Connections to input B
            // k = 2: Connections to input C
            // k = 3: Connections to output D
            for k in 0..4 {
                let pin = &gate.pins[k];
                let connections_to_input_a = &pin.connections_to_input_a;
                for &ref2 in connections_to_input_a {
                    let mut ref2 = ref2 as usize;
                    if ref2 > 0 {
                        ref2 += offset;
                    }

                    if k == 0 {
                        connect(&mut conn_a, ref1, None, ref2);
                    } else if k == 1 {
                        connect(&mut conn_b, ref1, Some(&mut conn_a), ref2);
                    } else if k == 2 {
                        connect(&mut conn_c, ref1, Some(&mut conn_a), ref2);
                    } else {
                        connect(&mut conn_d, ref1, Some(&mut conn_a), ref2);
                    }
                }

                let connections_to_input_b = &pin.connections_to_input_b;
                for &ref2 in connections_to_input_b {
                    let mut ref2 = ref2 as usize;
                    if ref2 > 0 {
                        ref2 += offset;
                    }

                    if k == 0 {
                        connect(&mut conn_a, ref1, Some(&mut conn_b), ref2);
                    } else if k == 1 {
                        connect(&mut conn_b, ref1, None, ref2);
                    } else if k == 2 {
                        connect(&mut conn_c, ref1, Some(&mut conn_b), ref2);
                    } else {
                        connect(&mut conn_d, ref1, Some(&mut conn_b), ref2);
                    }
                }

                let connections_to_input_c = &pin.connections_to_input_c;
                for &ref2 in connections_to_input_c {
                    let mut ref2 = ref2 as usize;
                    if ref2 > 0 {
                        ref2 += offset;
                    }

                    if k == 0 {
                        connect(&mut conn_a, ref1, Some(&mut conn_c), ref2);
                    } else if k == 1 {
                        connect(&mut conn_b, ref1, Some(&mut conn_c), ref2);
                    } else if k == 2 {
                        connect(&mut conn_c, ref1, None, ref2);
                    } else {
                        connect(&mut conn_d, ref1, Some(&mut conn_c), ref2);
                    }
                }
            }
        }

        // Compute the connections.
        // Here, we don't need the "+1" because the zero_ref is assumed
        // to be an XOR gate which is encoded to be the field element 0
        for &line in keccakf_program.iter() {
            let mut line = line as usize;
            let op = keccakf_gates[line].op;
            if line > 0 {
                line += offset;
            }

            match op {
                GateOperation::Xor => gate_op[line] = F::ZERO,
                GateOperation::XorAndp => gate_op[line] = F::ONE,
                _ => panic!("Invalid op: {op:?}"),
            }
        }
    }

    (conn_a, conn_b, conn_c, conn_d, gate_op)
}
