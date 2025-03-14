use std::{error::Error, fs};

use clap::{Arg, Command};
use p3_field::PrimeCharacteristicRing;
use p3_goldilocks::Goldilocks;
use serde::de::DeserializeOwned;

use zisk_pil::KeccakfTrace;

use proofman_common::{write_fixed_cols_bin, FixedColsInfo};

mod goldilocks_constants;
mod keccakf_types;

use goldilocks_constants::{GOLDILOCKS_GEN, GOLDILOCKS_K};
use keccakf_types::{Connections, Script};

type F = Goldilocks;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("keccakf_fixed_gen")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("script")
                .short('s')
                .long("script")
                .value_name("script_path")
                .help("Path to the script JSON file")
                .default_value("precompiles/keccakf/src/keccakf_script.json"),
        )
        .arg(
            Arg::new("connections")
                .short('c')
                .long("connections")
                .value_name("connections_path")
                .help("Path to the connections JSON file")
                .default_value("precompiles/keccakf/src/keccakf_connections.json"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("output_path")
                .help("Path to the output binary file")
                .default_value("precompiles/keccakf/src/keccakf_fixed.bin"),
        )
        .get_matches();

    let script_path = matches.get_one::<String>("script").unwrap();
    let connections_path = matches.get_one::<String>("connections").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();

    let n: usize = KeccakfTrace::<usize>::NUM_ROWS;
    let bits = log2(n);

    // Get the script and connections
    let script: Script = read_json(script_path)?;
    let connections: Connections = read_json(connections_path)?;

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

    write_fixed_cols_bin(
        output_file,
        "Zisk",
        "Keccakf",
        n as u64,
        &mut [conn_a, conn_b, conn_c, gate_op],
    );
    println!("CONN_A, CONN_B, CONN_C and GATE_OP columns written to {}", output_file);

    Ok(())
}

fn log2(n: usize) -> usize {
    let mut res = 0;
    let mut n = n;
    while n > 1 {
        n >>= 1;
        res += 1;
    }
    res
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
    let w = F::from_u64(subgroup_gen);
    let k = F::from_u64(cosets_gen);
    let ks = get_ks(k, 2);

    // Initialize the connections with the row identifiers
    let mut wi = F::ONE;
    let mut conn_a = vec![F::ONE; subgroup_order];
    let mut conn_b = vec![F::ONE; subgroup_order];
    let mut conn_c = vec![F::ONE; subgroup_order];
    for i in 0..subgroup_order {
        conn_a[i] = wi;
        conn_b[i] = wi * ks[0];
        conn_c[i] = wi * ks[1];
        wi *= w;
    }

    // Initialize the gate_op
    let mut gate_op = vec![F::ZERO; subgroup_order];

    // Compute the connections and gate_op
    for i in 0..num_slots {
        let offset = i * slot_size;
        for (j, connection) in connections.iter().enumerate() {
            let conn = &connection.0;
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
                "xor" => gate_op[ref_] = F::ZERO,
                "andp" => gate_op[ref_] = F::ONE,
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

fn connect(p1: &mut [F], i1: usize, p2: Option<&mut [F]>, i2: usize) {
    if let Some(p2) = p2 {
        std::mem::swap(&mut p1[i1], &mut p2[i2]);
    } else {
        p1.swap(i1, i2);
    }
}
