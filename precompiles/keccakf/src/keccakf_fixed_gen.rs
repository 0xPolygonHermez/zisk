use std::{env, error::Error, fs};

use p3_field::{AbstractField, PrimeField64};
use p3_goldilocks::Goldilocks;
use serde::de::DeserializeOwned;

mod keccakf_types;
mod goldilocks;

use keccakf_types::{Script, Connections};
use goldilocks::{GOLDILOCKS_GEN, GOLDILOCKS_K};

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
    let connections = connections.0;
    assert!(script.program.len() + 1 == connections.len());

    let slot_size = script.maxref;
    if slot_size >= n {
        panic!("The provided number of bits is too small to fit the script");
    }

    let num_slots = (n - 1) / slot_size;
    println!("num_slots: {}", num_slots);

    // Get the coset generators "ks" and the generator "w"
    let k = F::from_canonical_u64(GOLDILOCKS_K);
    let ks = get_ks(k, 2);
    let w = F::from_canonical_u64(GOLDILOCKS_GEN[bits]);

    // Initialize the connections with the row identifiers
    let mut wi = F::one();
    let mut conn_a = vec![F::one(); n];
    let mut conn_b = vec![F::one(); n];
    let mut conn_c = vec![F::one(); n];
    for i in 0..n {
        conn_a[i] = wi;
        conn_b[i] = wi * ks[0];
        conn_c[i] = wi * ks[1];
        wi *= w;
    }

    // Initialize the gate_op
    let mut gate_op = vec![F::zero(); n];

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

    // Serialize the data
    let mut data = Vec::new();
    data.extend(conn_a.iter().flat_map(|f| f.as_canonical_u64().to_le_bytes()));
    data.extend(conn_b.iter().flat_map(|f| f.as_canonical_u64().to_le_bytes()));
    data.extend(conn_c.iter().flat_map(|f| f.as_canonical_u64().to_le_bytes()));
    data.extend(gate_op.iter().flat_map(|f| f.as_canonical_u64().to_le_bytes()));

    // Write to binary
    let output_file = "precompiles/keccakf/src/keccakf_fixed.bin";
    write_binary(output_file, &data)?;
    println!("Fixed columns written to {}", output_file);

    Ok(())
}

fn write_binary(file_path: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
    fs::write(file_path, data)?;
    Ok(())
}

fn read_json<T: DeserializeOwned>(file_path: &str) -> Result<T, Box<dyn Error>> {
    let json_content = fs::read_to_string(file_path)?;
    let user: T = serde_json::from_str(&json_content)?;
    Ok(user)
}

fn get_ks(k: F, n: usize) -> Vec<F> {
    let mut ks = vec![k];
    for i in 1..n {
        ks.push(ks[i-1] * k);
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

