use path_clean::PathClean;
use std::path::Path;

mod babyjubjub_constants;
mod generator;
use babyjubjub_constants::{BABYJUBJUB_CHUNKS, BABYJUBJUB_CHUNK_BITS, BABYJUBJUB_PRIME};
use generator::{Equation, EquationConfig};

// BabyJubJub (twisted Edwards) complete point addition, circom-compatible.
//
// Curve: a*x^2 + y^2 = 1 + d*x^2*y^2 over Fr (BN254 scalar field), a = 168700, d = 168696.
//   tau = x1*x2*y1*y2
//   x3  = (x1*y2 + y1*x2) / (1 + d*tau)
//   y3  = (y1*y2 - a*x1*x2) / (1 - d*tau)
//
// Decomposed into 7 degree-<=2 polynomial identities over 16x16-bit chunks with carries.
// Scratch values (all reduced mod p):
//   A  = x1*x2,  B = y1*y2,  N = x1*y2 + y1*x2,  T = A*B (= tau),  DT = d*T.
//
// Sign convention (mirrors arith_eq bn254_curve):
//   plus  : "... - p*q + p*offset"  =>  q = _q / p + offset
//   minus : "... + p*q - p*offset"  =>  q = offset - _q / p
fn main() {
    let current_file_path = Path::new(file!());
    let current_dir = current_file_path.parent().expect("Error getting parent directory");
    let rust_code_path = current_dir.join("equations/");
    let pil_code_path = current_dir.join("../pil/equations/").clean();

    let config = EquationConfig {
        chunks: BABYJUBJUB_CHUNKS,
        chunk_bits: BABYJUBJUB_CHUNK_BITS,
        terms_by_clock: 2,
        ..Default::default()
    };

    let p = format!("0x{BABYJUBJUB_PRIME}");

    // EQ_A: A = x1*x2 mod p
    let mut eq = Equation::new(&config);
    eq.parse("x1*x2-A-p*qa+p*offset", &[("p", &p), ("offset", "0x10")]);
    eq.generate_rust_code_to_file(
        "BabyJubJubA",
        "x1,x2,A,qa",
        rust_code_path.join("babyjubjub_a.rs").to_str().unwrap(),
    );
    eq.generate_pil_code_to_file(
        "eq_babyjubjub_a",
        pil_code_path.join("babyjubjub_a.pil").to_str().unwrap(),
    );

    // EQ_B: B = y1*y2 mod p
    let mut eq = Equation::new(&config);
    eq.parse("y1*y2-B-p*qb+p*offset", &[("p", &p), ("offset", "0x10")]);
    eq.generate_rust_code_to_file(
        "BabyJubJubB",
        "y1,y2,B,qb",
        rust_code_path.join("babyjubjub_b.rs").to_str().unwrap(),
    );
    eq.generate_pil_code_to_file(
        "eq_babyjubjub_b",
        pil_code_path.join("babyjubjub_b.pil").to_str().unwrap(),
    );

    // EQ_N: Nx = x1*y2 + y1*x2 mod p  (named Nx to avoid clashing with the airtemplate's N degree)
    let mut eq = Equation::new(&config);
    eq.parse("x1*y2+y1*x2-Nx-p*qn+p*offset", &[("p", &p), ("offset", "0x10")]);
    eq.generate_rust_code_to_file(
        "BabyJubJubN",
        "x1,y2,y1,x2,Nx,qn",
        rust_code_path.join("babyjubjub_n.rs").to_str().unwrap(),
    );
    eq.generate_pil_code_to_file(
        "eq_babyjubjub_n",
        pil_code_path.join("babyjubjub_n.pil").to_str().unwrap(),
    );

    // EQ_T: T = A*B mod p   (T = tau)
    let mut eq = Equation::new(&config);
    eq.parse("A*B-T-p*qt+p*offset", &[("p", &p), ("offset", "0x10")]);
    eq.generate_rust_code_to_file(
        "BabyJubJubT",
        "A,B,T,qt",
        rust_code_path.join("babyjubjub_t.rs").to_str().unwrap(),
    );
    eq.generate_pil_code_to_file(
        "eq_babyjubjub_t",
        pil_code_path.join("babyjubjub_t.pil").to_str().unwrap(),
    );

    // EQ_DT: DT = d*T mod p
    let mut eq = Equation::new(&config);
    eq.parse("d*T-DT-p*qdt+p*offset", &[("d", "168696"), ("p", &p), ("offset", "0x10")]);
    eq.generate_rust_code_to_file(
        "BabyJubJubDt",
        "T,DT,qdt",
        rust_code_path.join("babyjubjub_dt.rs").to_str().unwrap(),
    );
    eq.generate_pil_code_to_file(
        "eq_babyjubjub_dt",
        pil_code_path.join("babyjubjub_dt.pil").to_str().unwrap(),
    );

    // EQ_X3: x3 + x3*DT - Nx == 0 mod p   =>   x3 = Nx / (1 + DT)
    let mut eq = Equation::new(&config);
    eq.parse("x3+x3*DT-Nx-p*qx+p*offset", &[("p", &p), ("offset", "0x10")]);
    eq.generate_rust_code_to_file(
        "BabyJubJubX3",
        "x3,DT,Nx,qx",
        rust_code_path.join("babyjubjub_x3.rs").to_str().unwrap(),
    );
    eq.generate_pil_code_to_file(
        "eq_babyjubjub_x3",
        pil_code_path.join("babyjubjub_x3.pil").to_str().unwrap(),
    );

    // EQ_Y3: y3 - y3*DT - B + a*A == 0 mod p   =>   y3 = (B - a*A) / (1 - DT)
    let mut eq = Equation::new(&config);
    eq.parse("y3-y3*DT-B+a*A+p*qy-p*offset", &[("a", "168700"), ("p", &p), ("offset", "0x40000")]);
    eq.generate_rust_code_to_file(
        "BabyJubJubY3",
        "y3,DT,B,A,qy",
        rust_code_path.join("babyjubjub_y3.rs").to_str().unwrap(),
    );
    eq.generate_pil_code_to_file(
        "eq_babyjubjub_y3",
        pil_code_path.join("babyjubjub_y3.pil").to_str().unwrap(),
    );
}
