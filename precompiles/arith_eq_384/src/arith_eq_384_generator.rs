use path_clean::PathClean;
use std::path::Path;

use precomp_arith_eq::generator::{Equation, EquationConfig};

mod arith_eq_384_constants;
use arith_eq_384_constants::{ARITH_EQ_384_CHUNKS, ARITH_EQ_384_CHUNK_BITS};

// cargo run --release --bin arith_eq_384_generator

fn main() {
    let current_file_path = Path::new(file!());
    let current_dir = current_file_path.parent().expect("Error getting parent directory");
    let rust_code_path = current_dir.join("equations/");
    let pil_code_path = current_dir.join("../pil/equations/").clean();

    let config = EquationConfig {
        chunks: ARITH_EQ_384_CHUNKS,
        chunk_bits: ARITH_EQ_384_CHUNK_BITS,
        terms_by_clock: 2,
        ..Default::default()
    };

    // ARITH_384_MOD

    let mut eq = Equation::new(&config);
    eq.parse(
        "x1*y1+x2-x3-q0*y2-q1*y2*p2_384",
        &[("p2_384", "0x1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")],
    );

    let rust_file = rust_code_path.join("arith384_mod.rs");
    eq.generate_rust_code_to_file(
        "Arith384Mod",
        "x1,y1,x2,y2,x3,q0,q1",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("arith384_mod.pil");
    eq.generate_pil_code_to_file("eq_arith384_mod", pil_file.to_str().unwrap());

    // BLS12_381

    // s - different points

    let mut eq = Equation::new(&config);
    eq.parse(
        "s*x2-s*x1-y2+y1-p*q0+p*offset",
        &[
            ("p", "0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab"),
            ("offset", "0x400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("bls12_381_curve_add.rs");
    eq.generate_rust_code_to_file(
        "Bls12_381CurveAdd",
        "x1,y1,x2,y2,s,q0",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("bls12_381_curve_add.pil");
    eq.generate_pil_code_to_file("eq_bls12_381_curve_add", pil_file.to_str().unwrap());

    // s - duplicate points

    let mut eq = Equation::new(&config);
    eq.parse(
        "2*s*y1-3*x1*x1+p*q0-p*offset",
        &[
            ("p", "0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab"),
            ("offset", "0x800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("bls12_381_curve_dbl.rs");
    eq.generate_rust_code_to_file("Bls12_381CurveDbl", "x1,y1,s,q0", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("bls12_381_curve_dbl.pil");
    eq.generate_pil_code_to_file("eq_bls12_381_curve_dbl", pil_file.to_str().unwrap());

    // x3

    let mut eq = Equation::new(&config);
    eq.parse(
        "s*s-x1-x2-x3-p*q1+p*offset",
        &[
            ("p", "0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab"),
            ("offset", "0x4"),
        ],
    );

    let rust_file = rust_code_path.join("bls12_381_curve_x3.rs");
    eq.generate_rust_code_to_file("Bls12_381CurveX3", "x1,x2,x3,s,q1", rust_file.to_str().unwrap());

    let pil_file = pil_code_path.join("bls12_381_curve_x3.pil");
    eq.generate_pil_code_to_file("eq_bls12_381_curve_x3", pil_file.to_str().unwrap());

    // y3

    let mut eq = Equation::new(&config);
    eq.parse(
        "s*x1-s*x3-y1-y3+p*q2-p*offset",
        &[
            ("p", "0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab"),
            ("offset", "0x400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("bls12_381_curve_y3.rs");
    eq.generate_rust_code_to_file(
        "Bls12_381CurveY3",
        "x1,y1,x3,y3,s,q2",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("bls12_381_curve_y3.pil");
    eq.generate_pil_code_to_file("eq_bls12_381_curve_y3", pil_file.to_str().unwrap());

    // x3 - complex addition

    let mut eq = Equation::new(&config);
    eq.parse(
        "x1+x2-x3-p*q1+p*offset",
        &[
            ("p", "0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab"),
            ("offset", "0x1"),
        ],
    );

    let rust_file = rust_code_path.join("bls12_381_complex_add_x3.rs");
    eq.generate_rust_code_to_file(
        "Bls12_381ComplexAddX3",
        "x1,x2,x3,q1",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("bls12_381_complex_add_x3.pil");
    eq.generate_pil_code_to_file("eq_bls12_381_complex_add_x3", pil_file.to_str().unwrap());

    // y3 - complex addition

    let mut eq = Equation::new(&config);
    eq.parse(
        "y1+y2-y3-p*q2+p*offset",
        &[
            ("p", "0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab"),
            ("offset", "0x1"),
        ],
    );

    let rust_file = rust_code_path.join("bls12_381_complex_add_y3.rs");
    eq.generate_rust_code_to_file(
        "Bls12_381ComplexAddY3",
        "y1,y2,y3,q2",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("bls12_381_complex_add_y3.pil");
    eq.generate_pil_code_to_file("eq_bls12_381_complex_add_y3", pil_file.to_str().unwrap());

    // x3 - complex subtraction

    let mut eq = Equation::new(&config);
    eq.parse(
        "x1-x2-x3+p*q1-p*offset",
        &[
            ("p", "0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab"),
            ("offset", "0x1"),
        ],
    );

    let rust_file = rust_code_path.join("bls12_381_complex_sub_x3.rs");
    eq.generate_rust_code_to_file(
        "Bls12_381ComplexSubX3",
        "x1,x2,x3,q1",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("bls12_381_complex_sub_x3.pil");
    eq.generate_pil_code_to_file("eq_bls12_381_complex_sub_x3", pil_file.to_str().unwrap());

    // y3 - complex subtraction

    let mut eq = Equation::new(&config);
    eq.parse(
        "y1-y2-y3+p*q2-p*offset",
        &[
            ("p", "0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab"),
            ("offset", "0x1"),
        ],
    );

    let rust_file = rust_code_path.join("bls12_381_complex_sub_y3.rs");
    eq.generate_rust_code_to_file(
        "Bls12_381ComplexSubY3",
        "y1,y2,y3,q2",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("bls12_381_complex_sub_y3.pil");
    eq.generate_pil_code_to_file("eq_bls12_381_complex_sub_y3", pil_file.to_str().unwrap());

    // x3 - complex multiplication

    let mut eq = Equation::new(&config);
    eq.parse(
        "x1*x2-y1*y2-x3+p*q1-p*offset",
        &[
            ("p", "0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab"),
            ("offset", "0x400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        ],
    );

    let rust_file = rust_code_path.join("bls12_381_complex_mul_x3.rs");
    eq.generate_rust_code_to_file(
        "Bls12_381ComplexMulX3",
        "x1,y1,x2,y2,x3,q1",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("bls12_381_complex_mul_x3.pil");
    eq.generate_pil_code_to_file("eq_bls12_381_complex_mul_x3", pil_file.to_str().unwrap());

    // y3 - complex multiplication

    let mut eq = Equation::new(&config);
    eq.parse(
        "y1*x2+x1*y2-y3-p*q2+p*offset",
        &[
            ("p", "0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab"),
            ("offset", "0x1"),
        ],
    );

    let rust_file = rust_code_path.join("bls12_381_complex_mul_y3.rs");
    eq.generate_rust_code_to_file(
        "Bls12_381ComplexMulY3",
        "x1,y1,x2,y2,y3,q2",
        rust_file.to_str().unwrap(),
    );

    let pil_file = pil_code_path.join("bls12_381_complex_mul_y3.pil");
    eq.generate_pil_code_to_file("eq_bls12_381_complex_mul_y3", pil_file.to_str().unwrap());
}
