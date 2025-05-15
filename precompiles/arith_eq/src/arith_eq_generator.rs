use path_clean::PathClean;
use std::path::Path;

mod arith_eq_constants;
mod generator;
use arith_eq_constants::{ARITH_EQ_CHUNKS, ARITH_EQ_CHUNK_BITS};
use generator::{Equation, EquationConfig};

fn main() {
    let current_file_path = Path::new(file!());
    let current_dir = current_file_path.parent().expect("Error getting parent directory");
    let rust_code_path = current_dir.join("equations/");
    let pil_code_path = current_dir.join("../pil/equations/").clean();

    let config = EquationConfig {
        chunks: ARITH_EQ_CHUNKS,
        chunk_bits: ARITH_EQ_CHUNK_BITS,
        terms_by_clock: 2,
        ..Default::default()
    };

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

    // BN254

    // s - different points

    let mut eq = Equation::new(&config);
    eq.parse(
        "s*x2-s*x1-y2+y1-p*q0+p*offset",
        &[
            ("p", "0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            ("offset", "0x80000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("bn254_curve_add.rs");
    eq.generate_rust_code_to_file("Bn254CurveAdd", "x1,y1,x2,y2,s,q0", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("bn254_curve_add.pil");
    eq.generate_pil_code_to_file("eq_bn254_curve_add", pil_file.to_str().unwrap());

    // s - duplicate points

    let mut eq = Equation::new(&config);
    eq.parse(
        "2*s*y1-3*x1*x1+p*q0-p*offset",
        &[
            ("p", "0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            ("offset", "0x100000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("bn254_curve_dbl.rs");
    eq.generate_rust_code_to_file("Bn254CurveDbl", "x1,y1,s,q0", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("bn254_curve_dbl.pil");
    eq.generate_pil_code_to_file("eq_bn254_curve_dbl", pil_file.to_str().unwrap());

    // x3

    let mut eq = Equation::new(&config);
    eq.parse(
        "s*s-x1-x2-x3-p*q1+p*offset",
        &[
            ("p", "0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            ("offset", "0x10"),
        ],
    );

    let rust_file = rust_code_path.join("bn254_curve_x3.rs");
    eq.generate_rust_code_to_file("Bn254CurveX3", "x1,x2,x3,s,q1", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("bn254_curve_x3.pil");
    eq.generate_pil_code_to_file("eq_bn254_curve_x3", pil_file.to_str().unwrap());

    // y3

    let mut eq = Equation::new(&config);
    eq.parse(
        "s*x1-s*x3-y1-y3+p*q2-p*offset",
        &[
            ("p", "0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            ("offset", "0x80000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("bn254_curve_y3.rs");
    eq.generate_rust_code_to_file("Bn254CurveY3", "x1,y1,x3,y3,s,q2", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("bn254_curve_y3.pil");
    eq.generate_pil_code_to_file("eq_bn254_curve_y3", pil_file.to_str().unwrap());

    // x3 - complex addition

    let mut eq = Equation::new(&config);
    eq.parse(
        "x1+x2-x3-p*q1+p*offset",
        &[
            ("p", "0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            ("offset", "0x8"),
        ],
    );

    let rust_file = rust_code_path.join("bn254_complex_add_x3.rs");
    eq.generate_rust_code_to_file("Bn254ComplexAddX3", "x1,x2,x3,q1", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("bn254_complex_add_x3.pil");
    eq.generate_pil_code_to_file("eq_bn254_complex_add_x3", pil_file.to_str().unwrap());

    // y3 - complex addition

    let mut eq = Equation::new(&config);
    eq.parse(
        "y1+y2-y3-p*q2+p*offset",
        &[
            ("p", "0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            ("offset", "0x8"),
        ],
    );

    let rust_file = rust_code_path.join("bn254_complex_add_y3.rs");
    eq.generate_rust_code_to_file("Bn254ComplexAddY3", "y1,y2,y3,q2", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("bn254_complex_add_y3.pil");
    eq.generate_pil_code_to_file("eq_bn254_complex_add_y3", pil_file.to_str().unwrap());

    // x3 - complex subtraction

    let mut eq = Equation::new(&config);
    eq.parse(
        "x1-x2-x3+p*q1-p*offset",
        &[
            ("p", "0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            ("offset", "0x8"),
        ],
    );

    let rust_file = rust_code_path.join("bn254_complex_sub_x3.rs");
    eq.generate_rust_code_to_file("Bn254ComplexSubX3", "x1,x2,x3,q1", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("bn254_complex_sub_x3.pil");
    eq.generate_pil_code_to_file("eq_bn254_complex_sub_x3", pil_file.to_str().unwrap());

    // y3 - complex subtraction

    let mut eq = Equation::new(&config);
    eq.parse(
        "y1-y2-y3+p*q2-p*offset",
        &[
            ("p", "0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            ("offset", "0x8"),
        ],
    );

    let rust_file = rust_code_path.join("bn254_complex_sub_y3.rs");
    eq.generate_rust_code_to_file("Bn254ComplexSubY3", "y1,y2,y3,q2", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("bn254_complex_sub_y3.pil");
    eq.generate_pil_code_to_file("eq_bn254_complex_sub_y3", pil_file.to_str().unwrap());

    // x3 - complex multiplication

    let mut eq = Equation::new(&config);
    eq.parse(
        "x1*x2-y1*y2-x3+p*q1-p*offset",
        &[
            ("p", "0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            ("offset", "0x80000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("bn254_complex_mul_x3.rs");
    eq.generate_rust_code_to_file(
        "Bn254ComplexMulX3",
        "x1,y1,x2,y2,x3,q1",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("bn254_complex_mul_x3.pil");
    eq.generate_pil_code_to_file("eq_bn254_complex_mul_x3", pil_file.to_str().unwrap());

    // y3 - complex multiplication

    let mut eq = Equation::new(&config);
    eq.parse(
        "y1*x2+x1*y2-y3-p*q2+p*offset",
        &[
            ("p", "0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
            ("offset", "0x8"),
        ],
    );

    let rust_file = rust_code_path.join("bn254_complex_mul_y3.rs");
    eq.generate_rust_code_to_file(
        "Bn254ComplexMulY3",
        "x1,y1,x2,y2,y3,q2",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("bn254_complex_mul_y3.pil");
    eq.generate_pil_code_to_file("eq_bn254_complex_mul_y3", pil_file.to_str().unwrap());
}
