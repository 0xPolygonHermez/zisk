use std::path::Path;

mod generator;
use generator::{Arith256Equation, Arith256EquationConfig};

fn main() {
    let current_file_path = Path::new(file!());
    let current_dir = current_file_path.parent().expect("Error getting parent directory");

    let config = Arith256EquationConfig {
        chunks: 16,
        chunk_bits: 16,
        terms_by_clock: 2,
        ..Default::default()
    };

    let mut eq = Arith256Equation::new(&config);
    eq.parse(
        "x1*y1+x2-x3-y3*p2_256",
        &[("p2_256", "0x10000000000000000000000000000000000000000000000000000000000000000")],
    );

    let rust_file = current_dir.join("helpers/eq_arith256.rs");
    eq.generate_rust_code_to_file("EqArith256", "x1,y1,x2,x3,y3", rust_file.to_str().unwrap());

    let pil_file = current_dir.join("../pil/eq_arith256.pil");
    eq.generate_pil_code_to_file("eq_arith256", pil_file.to_str().unwrap());

    eq.parse(
        "x1*y1+x2-x3-q0*y2-q1*y2*p2_256",
        &[("p2_256", "0x10000000000000000000000000000000000000000000000000000000000000000")],
    );

    let rust_file = current_dir.join("helpers/eq_arith256_mod.rs");
    eq.generate_rust_code_to_file(
        "EqArith256Mod",
        "x1,y1,x2,y2,x3,q0,q1",
        rust_file.to_str().unwrap(),
    );

    let pil_file = current_dir.join("../pil/eq_arith256_mod.pil");
    eq.generate_pil_code_to_file("eq_arith256_mod", pil_file.to_str().unwrap());

    // SECP256K1

    // s - different points

    let mut eq = Arith256Equation::new(&config);
    eq.parse(
        "s*x2-s*x1-y2+y1-p*q0+p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x20000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = current_dir.join("helpers/eq_secp256k1_add.rs");
    eq.generate_rust_code_to_file(
        "EqSecp256k1Add",
        "x1,y1,x2,y2,s,q0",
        rust_file.to_str().unwrap(),
    );

    let pil_file = current_dir.join("../pil/eq_secp256k1_add.pil");
    eq.generate_pil_code_to_file("eq_secp256k1_add", pil_file.to_str().unwrap());

    // s - duplicate points

    let mut eq = Arith256Equation::new(&config);
    eq.parse(
        "2*s*y1-3*x1*x1+p*q0-p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x40000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = current_dir.join("helpers/eq_secp256k1_dbl.rs");
    eq.generate_rust_code_to_file("EqSecp256k1Dbl", "x1,y1,s,q0", rust_file.to_str().unwrap());

    let pil_file = current_dir.join("../pil/eq_secp256k1_dbl.pil");
    eq.generate_pil_code_to_file("eq_secp256k1_dbl", pil_file.to_str().unwrap());

    // x3

    let mut eq = Arith256Equation::new(&config);
    eq.parse(
        "s*s-x1-x2-x3-p*q1+p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x4"),
        ],
    );

    let rust_file = current_dir.join("helpers/eq_secp256k1_x3.rs");
    eq.generate_rust_code_to_file("EqSecp256k1X3", "x1,x2,x3,s,q1", rust_file.to_str().unwrap());

    let pil_file = current_dir.join("../pil/eq_secp256k1_x3.pil");
    eq.generate_pil_code_to_file("eq_secp256k1_x3", pil_file.to_str().unwrap());

    // y3

    let mut eq = Arith256Equation::new(&config);
    eq.parse(
        "s*x1-s*x3-y1-y3+p*q2-p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x20000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = current_dir.join("helpers/eq_secp256k1_y3.rs");
    eq.generate_rust_code_to_file("EqSecp256k1Y3", "x1,y1,x3,y3,s,q2", rust_file.to_str().unwrap());

    let pil_file = current_dir.join("../pil/eq_secp256k1_y3.pil");
    eq.generate_pil_code_to_file("eq_secp256k1_y3", pil_file.to_str().unwrap());
}
