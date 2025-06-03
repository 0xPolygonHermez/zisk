use std::{collections::HashMap, error::Error, fs};

use clap::{Arg, Command};
use fields::{Field, Goldilocks, PrimeField64};
use serde::de::DeserializeOwned;

use zisk_pil::Sha256fTrace;

use precompiles_common::{get_ks, log2, GOLDILOCKS_GEN, GOLDILOCKS_K};
use proofman_common::{write_fixed_cols_bin, FixedColsInfo};

use precomp_sha256f::{Gate, GateOp, ADD_GATE_OP, CH_GATE_OP, MAJ_GATE_OP, XOR_GATE_OP};

type F = Goldilocks;

type FixedCols = (Vec<F>, Vec<F>, Vec<F>, Vec<F>, Vec<F>, Vec<F>);

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("sha256f_fixed_gen")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("gates")
                .short('c')
                .long("gates")
                .value_name("gates_path")
                .help("Path to the gates JSON file")
                .default_value("precompiles/sha256f/src/sha256f_gates.json"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("output_path")
                .help("Path to the output binary file")
                .default_value("precompiles/sha256f/src/sha256f_fixed.bin"),
        )
        .get_matches();

    let gates_path = matches.get_one::<String>("gates").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();

    let n: usize = Sha256fTrace::<usize>::NUM_ROWS;
    let bits = log2(n);

    // Get the gates
    let gates: Vec<Gate> = read_json(gates_path)?;

    // Get the subgroup generator and coset generator
    let subgroup_gen = GOLDILOCKS_GEN[bits];
    let cosets_gen = GOLDILOCKS_K;

    // Generate the columns
    let (conn_a, conn_b, conn_c, conn_d, gate_op, carry_enabled) =
        cols_gen(n, subgroup_gen, cosets_gen, gates);

    // Serialize the columns and write them to a binary file
    let conn_a = FixedColsInfo::new("Sha256f.CONN_A", None, conn_a);
    let conn_b = FixedColsInfo::new("Sha256f.CONN_B", None, conn_b);
    let conn_c = FixedColsInfo::new("Sha256f.CONN_C", None, conn_c);
    let conn_d = FixedColsInfo::new("Sha256f.CONN_D", None, conn_d);
    let gate_op = FixedColsInfo::new("Sha256f.GATE_OP", None, gate_op);
    let carry_enabled = FixedColsInfo::new("Sha256f.CARRY_ENABLED", None, carry_enabled);

    write_fixed_cols_bin(
        output_file,
        "Zisk",
        "Sha256f",
        n as u64,
        &mut [conn_a, conn_b, conn_c, conn_d, gate_op, carry_enabled],
    );
    println!(
        "CONN_A, CONN_B, CONN_C, CONN_D, GATE_OP and CARRY_ENABLED columns written to {}",
        output_file
    );

    Ok(())
}

fn read_json<T: DeserializeOwned>(file_path: &str) -> Result<T, Box<dyn Error>> {
    let json_content = fs::read_to_string(file_path)?;
    let user: T = serde_json::from_str(&json_content)?;
    Ok(user)
}

fn cols_gen(
    subgroup_order: usize,
    subgroup_gen: u64,
    cosets_gen: u64,
    gates: Vec<Gate>,
) -> FixedCols {
    fn connect(c1: &mut [F], i1: usize, c2: Option<&mut Vec<F>>, i2: usize, offset: usize) {
        let adjust = |i| if i > 0 { i + offset } else { i };
        let row1 = adjust(i1);
        let row2 = adjust(i2);

        match c2 {
            Some(c2) => std::mem::swap(&mut c1[row1], &mut c2[row2]),
            None => c1.swap(row1, row2),
        }
    }

    // Check that the subgroup order is sufficiently large
    let circuit_size = gates.len() - 1;
    if circuit_size >= subgroup_order {
        panic!("The provided number of bits is too small to fit the script");
    }

    // Get the number of circuits we can generate
    let num_circuits = (subgroup_order - 1) / circuit_size;

    // Get the coset generators "ks" and the generator "w"
    let w = F::from_u64(subgroup_gen);
    let k = F::from_u64(cosets_gen);
    let ks = get_ks(k, 3);

    // Initialize the gates with the row identifiers
    let mut wi = F::ONE;
    let mut conn_a = vec![F::ZERO; subgroup_order];
    let mut conn_b = vec![F::ZERO; subgroup_order];
    let mut conn_c = vec![F::ZERO; subgroup_order];
    let mut conn_d = vec![F::ZERO; subgroup_order];
    for i in 0..subgroup_order {
        conn_a[i] = wi;
        conn_b[i] = wi * ks[0];
        conn_c[i] = wi * ks[1];
        conn_d[i] = wi * ks[2];
        wi *= w;
    }

    // Initialize the gate_op and carry_enabled vectors
    let mut gate_op = vec![F::ZERO; subgroup_order];
    let mut carry_enabled = vec![F::ZERO; subgroup_order];

    // First row is reserved for constant signals 0,1
    gate_op[0] = F::from_u8(XOR_GATE_OP);
    carry_enabled[0] = F::ZERO;

    // Compute the gates and gate_op
    for i in 0..num_circuits {
        let offset = i * circuit_size;

        let mut wires: HashMap<usize, [usize; 2]> = HashMap::new();
        // Map explanation:
        //  · key    -> wire_idx
        //  · val[0] -> a,b,c or d (0,1,2 or 3)
        //  · val[1] -> gate_idx
        wires.insert(0, [0, 0]);
        wires.insert(1, [1, 0]);
        // First gate is XOR(0,1,0) = 1 => a = c = 0, b = d = 1
        connect(&mut conn_a, 0, Some(&mut conn_c), 0, 0);
        connect(&mut conn_b, 0, Some(&mut conn_d), 0, 0);
        for (j, gate) in gates.iter().enumerate().skip(1) {
            let conn = gate.connections;
            for (k, &wire) in conn.iter().enumerate() {
                if !wires.contains_key(&wire) {
                    // If the wire is not in the map, insert it with the current gate
                    wires.insert(wire, [k, j]);
                } else {
                    // Otherwise, connect the existing wire to the current gate
                    // and insert the wire with the current gate
                    let (c1, c2) = match (wires[&wire][0], k) {
                        (0, 0) => (&mut conn_a, None),
                        (0, 1) => (&mut conn_a, Some(&mut conn_b)),
                        (0, 2) => (&mut conn_a, Some(&mut conn_c)),
                        (0, 3) => (&mut conn_a, Some(&mut conn_d)),
                        (1, 0) => (&mut conn_b, Some(&mut conn_a)),
                        (1, 1) => (&mut conn_b, None),
                        (1, 2) => (&mut conn_b, Some(&mut conn_c)),
                        (1, 3) => (&mut conn_b, Some(&mut conn_d)),
                        (2, 0) => (&mut conn_c, Some(&mut conn_a)),
                        (2, 1) => (&mut conn_c, Some(&mut conn_b)),
                        (2, 2) => (&mut conn_c, None),
                        (2, 3) => (&mut conn_c, Some(&mut conn_d)),
                        (3, 0) => (&mut conn_d, Some(&mut conn_a)),
                        (3, 1) => (&mut conn_d, Some(&mut conn_b)),
                        (3, 2) => (&mut conn_d, Some(&mut conn_c)),
                        (3, 3) => (&mut conn_d, None),
                        _ => panic!("Invalid wire connection"),
                    };

                    connect(c1, wires[&wire][1], c2, j, offset);
                    wires.insert(wire, [k, j]);
                }
            }

            let mut row = j;
            if j > 0 {
                row += offset;
            }
            match gate.op {
                GateOp::xor => {
                    gate_op[row] = F::from_u8(XOR_GATE_OP);
                    carry_enabled[row] = F::ZERO;
                }
                GateOp::ch => {
                    gate_op[row] = F::from_u8(CH_GATE_OP);
                    carry_enabled[row] = F::ZERO;
                }
                GateOp::maj => {
                    gate_op[row] = F::from_u8(MAJ_GATE_OP);
                    carry_enabled[row] = F::ZERO;
                }
                GateOp::add => {
                    gate_op[row] = F::from_u8(ADD_GATE_OP);
                    carry_enabled[row] = F::ONE;
                }
            }
        }
    }

    (conn_a, conn_b, conn_c, conn_d, gate_op, carry_enabled)
}
