mod test_data;
use test_data::{get_secp256k1_add_test_data, get_secp256k1_dbl_test_data};

mod equations;
mod executors;
use lib_c::add_point_ec_p_c;
use std::time::Instant;

// cargo run --release --features="test_data" --bin arith_eq_test_c

fn main() {
    // Test addition against expected results
    let mut index = 0;
    while let Some((p1, p2, p3)) = get_secp256k1_add_test_data(index) {
        println!("testing index secp256k1_add #{} ....", index);
        println!("p1: {:?}", p1);
        println!("p2: {:?}", p2);
        println!("p3: {:?}", p3);
        let mut p4: [u64; 8] = [0; 8];
        let result = add_point_ec_p_c(0, &p1, &p2, &mut p4);
        if result != 0 {
            panic!("Error calling add_point_ec_p_c() result={}\np1=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x} )\np2=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x} )\nexpected_p3=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x} )\nresult_p3=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x})",
                result,
                p1[3], p1[2], p1[1], p1[0], p1[7], p1[6], p1[5], p1[4],
                p2[3], p2[2], p2[1], p2[0], p2[7], p2[6], p2[5], p2[4],
                p3[3], p3[2], p3[1], p3[0], p3[7], p3[6], p3[5], p3[4],
                p4[3], p4[2], p4[1], p4[0], p4[7], p4[6], p4[5], p4[4],
            );
        }
        if p4 != p3 {
            panic!("Error called add_point_ec_p_c() but p3 did not match result={}\np1=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x} )\np2=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x} )\nexpected_p3=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x} )\nresult_p3=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x})",
                result,
                p1[3], p1[2], p1[1], p1[0], p1[7], p1[6], p1[5], p1[4],
                p2[3], p2[2], p2[1], p2[0], p2[7], p2[6], p2[5], p2[4],
                p3[3], p3[2], p3[1], p3[0], p3[7], p3[6], p3[5], p3[4],
                p4[3], p4[2], p4[1], p4[0], p4[7], p4[6], p4[5], p4[4],
            );
        }
        index += 1;
    }

    // Test double against expected results
    index = 0;
    while let Some((p1, p3)) = get_secp256k1_dbl_test_data(index) {
        println!("testing index secp256k1_dbl #{} ....", index);
        println!("p1: {:?}", p1);
        println!("p3: {:?}", p3);
        let mut p4: [u64; 8] = [0; 8];
        let result = add_point_ec_p_c(1, &p1, &p1, &mut p4);
        if result != 0 {
            panic!("Error calling add_point_ec_p_c(dbl) result={}\np1=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x} )\nexpected_p3=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x} )\nresult_p3=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x})",
                result,
                p1[3], p1[2], p1[1], p1[0], p1[7], p1[6], p1[5], p1[4],
                p3[3], p3[2], p3[1], p3[0], p3[7], p3[6], p3[5], p3[4],
                p4[3], p4[2], p4[1], p4[0], p4[7], p4[6], p4[5], p4[4],
            );
        }
        if p4 != p3 {
            panic!("Error called add_point_ec_p_c(dbl) but p3 did not match result={}\np1=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x} )\nexpected_p3=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x} )\nresult_p3=( {:x}:{:x}:{:x}:{:x}, {:x}:{:x}:{:x}:{:x})",
                result,
                p1[3], p1[2], p1[1], p1[0], p1[7], p1[6], p1[5], p1[4],
                p3[3], p3[2], p3[1], p3[0], p3[7], p3[6], p3[5], p3[4],
                p4[3], p4[2], p4[1], p4[0], p4[7], p4[6], p4[5], p4[4],
            );
        }
        index += 1;
    }

    // Run the first test a million times to measure performance
    if let Some((p1, p2, _p3)) = get_secp256k1_add_test_data(0) {
        let mut p4: [u64; 8] = [0; 8];
        let start = Instant::now();
        for _ in 0..1000000 {
            let _result = add_point_ec_p_c(0, &p1, &p2, &mut p4);
        }
        let duration = start.elapsed();
        let secs = duration.as_secs_f64();
        let tp = 1_f64 / secs;
        println!("Duration = {:.4} sec, TP = {:.4} M/sec", secs, tp);
    }
}
