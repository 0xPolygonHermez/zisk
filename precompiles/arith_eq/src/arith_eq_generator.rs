use path_clean::PathClean;
use std::path::Path;

mod generator;
use generator::{Equation, EquationConfig};

fn main() {
    let current_file_path = Path::new(file!());
    let current_dir = current_file_path.parent().expect("Error getting parent directory");
    let rust_code_path = current_dir.join("equations/");
    let pil_code_path = current_dir.join("../pil/equations/").clean();

    let config =
        EquationConfig { chunks: 16, chunk_bits: 16, terms_by_clock: 2, ..Default::default() };

    let mut eq = Equation::new(&config);
    eq.parse(
        "x1*y1+x2-x3-y3*p2_256",
        &[("p2_256", "0x10000000000000000000000000000000000000000000000000000000000000000")],
    );

    let rust_file = rust_code_path.join("arith256.rs");
    eq.generate_rust_code_to_file("Arith256", "x1,y1,x2,x3,y3", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("arith256.pil");
    eq.generate_pil_code_to_file("eq_arith256", pil_file.to_str().unwrap());

    let mut eq = Equation::new(&config);
    eq.parse(
        "x1*y1+x2-x3-q0*y2-q1*y2*p2_256",
        &[("p2_256", "0x10000000000000000000000000000000000000000000000000000000000000000")],
    );

    let rust_file = rust_code_path.join("arith256_mod.rs");
    eq.generate_rust_code_to_file(
        "Arith256Mod",
        "x1,y1,x2,y2,x3,q0,q1",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("arith256_mod.pil");
    eq.generate_pil_code_to_file("eq_arith256_mod", pil_file.to_str().unwrap());

    // SECP256K1

    // s - different points

    let mut eq = Equation::new(&config);
    eq.parse(
        "s*x2-s*x1-y2+y1-p*q0+p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x20000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("secp256k1_add.rs");
    eq.generate_rust_code_to_file("Secp256k1Add", "x1,y1,x2,y2,s,q0", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("secp256k1_add.pil");
    eq.generate_pil_code_to_file("eq_secp256k1_add", pil_file.to_str().unwrap());

    // s - duplicate points

    let mut eq = Equation::new(&config);
    eq.parse(
        "2*s*y1-3*x1*x1+p*q0-p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x40000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("secp256k1_dbl.rs");
    eq.generate_rust_code_to_file("Secp256k1Dbl", "x1,y1,s,q0", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("secp256k1_dbl.pil");
    eq.generate_pil_code_to_file("eq_secp256k1_dbl", pil_file.to_str().unwrap());

    // x3

    let mut eq = Equation::new(&config);
    eq.parse(
        "s*s-x1-x2-x3-p*q1+p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x4"),
        ],
    );

    let rust_file = rust_code_path.join("secp256k1_x3.rs");
    eq.generate_rust_code_to_file("Secp256k1X3", "x1,x2,x3,s,q1", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("secp256k1_x3.pil");
    eq.generate_pil_code_to_file("eq_secp256k1_x3", pil_file.to_str().unwrap());

    // y3

    let mut eq = Equation::new(&config);
    eq.parse(
        "s*x1-s*x3-y1-y3+p*q2-p*offset",
        &[
            ("p", "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
            ("offset", "0x20000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("secp256k1_y3.rs");
    eq.generate_rust_code_to_file("Secp256k1Y3", "x1,y1,x3,y3,s,q2", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("secp256k1_y3.pil");
    eq.generate_pil_code_to_file("eq_secp256k1_y3", pil_file.to_str().unwrap());
}
