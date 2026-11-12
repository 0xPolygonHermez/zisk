use path_clean::PathClean;
use std::path::{Path, PathBuf};

mod babyjubjub_constants;
mod ed25519_constants;
mod twisted_edwards_constants;
use babyjubjub_constants::BABYJUBJUB_PRIME;
use ed25519_constants::{ED25519_D, ED25519_PRIME};
use twisted_edwards_constants::{TWISTED_EDWARDS_CHUNKS, TWISTED_EDWARDS_CHUNK_BITS};
use zisk_precomp_arith_eq::generator::{Equation, EquationConfig};

// Twisted-Edwards complete point addition, one equation set per curve.
//
// Curve: a*x^2 + y^2 = 1 + d*x^2*y^2 over Fp.
//   tau = x1*x2*y1*y2
//   x3  = (x1*y2 + y1*x2) / (1 + d*tau)
//   y3  = (y1*y2 - a*x1*x2) / (1 - d*tau)
//
// Decomposed into 7 degree-<=2 polynomial identities over 16x16-bit chunks with carries.
// Scratch values (all reduced mod p):
//   A  = x1*x2,  B = y1*y2,  N = x1*y2 + y1*x2,  T = A*B (= tau),  DT = d*T.
//
// Only EQ_DT (d) and EQ_Y3 (a) depend on the curve parameters beyond p.
//
// Sign convention (mirrors arith_eq bn254_curve):
//   plus  : "... - p*q + p*offset"  =>  q = _q / p + offset
//   minus : "... + p*q - p*offset"  =>  q = offset - _q / p
struct Curve<'a> {
    /// Prefix of the generated files (`<file>_a.rs`, `<file>_a.pil`, ...).
    file: &'a str,
    /// Prefix of the generated Rust structs (`<name>A`, `<name>B`, ...).
    name: &'a str,
    /// Prime as a "0x..." string.
    p: String,
    /// EQ_DT parameters: d.
    d: &'a str,
    /// EQ_Y3 equation (encodes a) and its offset.
    y3_eq: &'a str,
    y3_params: &'a [(&'a str, &'a str)],
}

/// (file suffix, equation, parameters, Rust struct suffix, Rust arguments)
type EquationSpec<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)], &'a str, &'a str);

fn generate(curve: &Curve, config: &EquationConfig, rust_code_path: &Path, pil_code_path: &Path) {
    let p = curve.p.as_str();
    let dt_params = [("d", curve.d), ("p", p), ("offset", "0x10")];
    let offset_params = [("p", p), ("offset", "0x10")];
    let mut y3_params: Vec<(&str, &str)> = curve.y3_params.to_vec();
    y3_params.insert(y3_params.len() - 1, ("p", p));

    let eqs: [EquationSpec; 7] = [
        // EQ_A: A = x1*x2 mod p
        ("a", "x1*x2-A-p*qa+p*offset", &offset_params, "A", "x1,x2,A,qa"),
        // EQ_B: B = y1*y2 mod p
        ("b", "y1*y2-B-p*qb+p*offset", &offset_params, "B", "y1,y2,B,qb"),
        // EQ_N: Nx = x1*y2 + y1*x2 mod p  (named Nx to avoid clashing with the airtemplate's N)
        ("n", "x1*y2+y1*x2-Nx-p*qn+p*offset", &offset_params, "N", "x1,y2,y1,x2,Nx,qn"),
        // EQ_T: T = A*B mod p   (T = tau)
        ("t", "A*B-T-p*qt+p*offset", &offset_params, "T", "A,B,T,qt"),
        // EQ_DT: DT = d*T mod p
        ("dt", "d*T-DT-p*qdt+p*offset", &dt_params, "Dt", "T,DT,qdt"),
        // EQ_X3: x3 + x3*DT - Nx == 0 mod p   =>   x3 = Nx / (1 + DT)
        ("x3", "x3+x3*DT-Nx-p*qx+p*offset", &offset_params, "X3", "x3,DT,Nx,qx"),
        // EQ_Y3: y3 - y3*DT - B + a*A == 0 mod p   =>   y3 = (B - a*A) / (1 - DT)
        ("y3", curve.y3_eq, &y3_params, "Y3", "y3,DT,B,A,qy"),
    ];

    for (suffix, equation, params, struct_suffix, args) in eqs {
        let mut eq = Equation::new(config);
        eq.parse(equation, params);
        let file = format!("{}_{suffix}", curve.file);
        eq.generate_rust_code_to_file(
            &format!("{}{struct_suffix}", curve.name),
            args,
            rust_code_path.join(format!("{file}.rs")).to_str().unwrap(),
        );
        eq.generate_pil_code_to_file(
            &format!("eq_{file}"),
            pil_code_path.join(format!("{file}.pil")).to_str().unwrap(),
        );
    }
}

fn main() {
    let current_file_path = Path::new(file!());
    let current_dir = current_file_path.parent().expect("Error getting parent directory");
    let rust_code_path: PathBuf = current_dir.join("equations/");
    let pil_code_path: PathBuf = current_dir.join("../pil/equations/").clean();

    let config = EquationConfig {
        chunks: TWISTED_EDWARDS_CHUNKS,
        chunk_bits: TWISTED_EDWARDS_CHUNK_BITS,
        terms_by_clock: 2,
        ..Default::default()
    };

    // BabyJubJub (circomlib): p = BN254 Fr, a = 168700, d = 168696.
    // a*A/p reaches ~2^17.4, hence the larger EQ_Y3 offset.
    generate(
        &Curve {
            file: "babyjubjub",
            name: "BabyJubJub",
            p: format!("0x{BABYJUBJUB_PRIME}"),
            d: "168696",
            y3_eq: "y3-y3*DT-B+a*A+p*qy-p*offset",
            y3_params: &[("a", "168700"), ("offset", "0x40000")],
        },
        &config,
        &rust_code_path,
        &pil_code_path,
    );

    // Ed25519 (RFC 8032): p = 2^255 - 19, a = -1, d = -121665/121666.
    // With a = -1, "+a*A" becomes "-A" and the EQ_Y3 quotient stays small, so offset 0x10.
    let ed25519_d = format!("0x{ED25519_D}");
    generate(
        &Curve {
            file: "ed25519",
            name: "Ed25519",
            p: format!("0x{ED25519_PRIME}"),
            d: &ed25519_d,
            y3_eq: "y3-y3*DT-B-A+p*qy-p*offset",
            y3_params: &[("offset", "0x10")],
        },
        &config,
        &rust_code_path,
        &pil_code_path,
    );
}
