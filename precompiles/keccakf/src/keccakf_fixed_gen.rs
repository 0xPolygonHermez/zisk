use std::{env, error::Error, fs};

use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use serde::de::DeserializeOwned;

use proofman_common::{write_fixed_cols_bin, FixedColsInfo};

mod goldilocks_constants;
mod keccakf_types;

use goldilocks_constants::{GOLDILOCKS_GEN, GOLDILOCKS_K};
use keccakf_types::{Connections, Script};

type F = Goldilocks;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: keccakf_fixed_gen <bits>");
        return Err("A number of bits is required".into());
    }

    let bits: usize = args[1].parse().map_err(|_| "Bits should be a byte")?;
    let n: usize = 1 << bits;

    // Get the script and connections
    let script: Script = read_json("precompiles/keccakf/src/keccakf_script.json")?;
    let connections: Connections = read_json("precompiles/keccakf/src/keccakf_connections.json")?;

    // Get the subgroup generator and coset generator
    let subgroup_gen = GOLDILOCKS_GEN[bits];
    let cosets_gen = GOLDILOCKS_K;

    // Generate the columns
    let (conn_a, conn_b, conn_c, gate_op) =
        cols_gen(n, subgroup_gen, cosets_gen, connections, script);

    // Serialize the columns and write them to a binary file
    let conn_a = FixedColsInfo::new("Keccakf.CONN_A", None, conn_a);
    let conn_b = FixedColsInfo::new("Keccakf.CONN_B", None, conn_b);
    let conn_c = FixedColsInfo::new("Keccakf.CONN_C", None, conn_c);
    let gate_op = FixedColsInfo::new("Keccakf.GATE_OP", None, gate_op);

    let output_file = format!("precompiles/keccakf/src/keccakf_fixed_{}.bin", bits);
    write_fixed_cols_bin(
        &output_file,
        "Zisk",
        "Keccakf",
        n as u64,
        &mut [conn_a, conn_b, conn_c, gate_op],
    );
    println!("CONN_A, CONN_B, CONN_C and GATE_OP columns written to {}", output_file);

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
    connections: Connections,
    script: Script,
) -> (Vec<F>, Vec<F>, Vec<F>, Vec<F>) {
    // Check the connections and the script are well-formed
    let connections = connections.0;
    assert!(script.program.len() + 1 == connections.len());

    // Check that the subgroup order is sufficiently large
    let slot_size = script.maxref;
    if slot_size >= subgroup_order {
        panic!("The provided number of bits is too small to fit the script");
    }

    // Get the number of slots we can generate
    let num_slots = (subgroup_order - 1) / slot_size;

    // Get the coset generators "ks" and the generator "w"
    let w = F::from_canonical_u64(subgroup_gen);
    let k = F::from_canonical_u64(cosets_gen);
    let ks = get_ks(k, 2);

    // Initialize the connections with the row identifiers
    let mut wi = F::one();
    let mut conn_a = vec![F::one(); subgroup_order];
    let mut conn_b = vec![F::one(); subgroup_order];
    let mut conn_c = vec![F::one(); subgroup_order];
    for i in 0..subgroup_order {
        conn_a[i] = wi;
        conn_b[i] = wi * ks[0];
        conn_c[i] = wi * ks[1];
        wi *= w;
    }

    // Initialize the gate_op
    let mut gate_op = vec![F::zero(); subgroup_order];

    // Compute the connections and gate_op
    for i in 0..num_slots {
        let offset = i * slot_size;
        for j in 0..connections.len() {
            let conn = &connections[j].0;
            let mut ref1 = j;
            if j > 0 {
                ref1 += offset;
            }

            if conn.contains_key("A") {
                for k in 0..conn["A"].len() {
                    let peer = &conn["A"][k];
                    let mut ref2 = peer.1;
                    if ref2 > 0 {
                        ref2 += offset;
                    }

                    let peer_type = peer.0.clone();
                    match peer_type.as_str() {
                        "A" => connect(&mut conn_a, ref1, None, ref2),
                        "B" => connect(&mut conn_a, ref1, Some(&mut conn_b), ref2),
                        "C" => connect(&mut conn_a, ref1, Some(&mut conn_c), ref2),
                        _ => panic!("Invalid peer type: {}", peer_type),
                    }
                }
            }

            if conn.contains_key("B") {
                for k in 0..conn["B"].len() {
                    let peer = &conn["B"][k];
                    let mut ref2 = peer.1;
                    if ref2 > 0 {
                        ref2 += offset;
                    }

                    let peer_type = peer.0.clone();
                    match peer_type.as_str() {
                        "A" => connect(&mut conn_b, ref1, Some(&mut conn_a), ref2),
                        "B" => connect(&mut conn_b, ref1, None, ref2),
                        "C" => connect(&mut conn_b, ref1, Some(&mut conn_c), ref2),
                        _ => panic!("Invalid peer type: {}", peer_type),
                    }
                }
            }

            if conn.contains_key("C") {
                for k in 0..conn["C"].len() {
                    let peer = &conn["C"][k];
                    let mut ref2 = peer.1;
                    if ref2 > 0 {
                        ref2 += offset;
                    }

                    let peer_type = peer.0.clone();
                    match peer_type.as_str() {
                        "A" => connect(&mut conn_c, ref1, Some(&mut conn_a), ref2),
                        "B" => connect(&mut conn_c, ref1, Some(&mut conn_b), ref2),
                        "C" => connect(&mut conn_c, ref1, None, ref2),
                        _ => panic!("Invalid peer type: {}", peer_type),
                    }
                }
            }
        }

        for j in 0..script.program.len() {
            let line = &script.program[j];
            let mut ref_ = line.ref_;
            if ref_ > 0 {
                ref_ += offset;
            }

            match line.op.as_str() {
                "xor" => gate_op[ref_] = F::one(),
                "andp" => gate_op[ref_] = F::zero(),
                _ => panic!("Invalid op: {}", line.op),
            }
        }
    }

    (conn_a, conn_b, conn_c, gate_op)
}

fn get_ks(k: F, n: usize) -> Vec<F> {
    let mut ks = vec![k];
    for i in 1..n {
        ks.push(ks[i - 1] * k);
    }
    ks
}

fn connect(p1: &mut Vec<F>, i1: usize, p2: Option<&mut Vec<F>>, i2: usize) {
    if let Some(p2) = p2 {
        std::mem::swap(&mut p1[i1], &mut p2[i2]);
    } else {
        p1.swap(i1, i2);
    }
}
