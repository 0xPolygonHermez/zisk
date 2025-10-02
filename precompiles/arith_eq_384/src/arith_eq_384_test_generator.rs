mod test_data;

use test_data::{
    get_arith384_mod_test_data, get_bls12_381_complex_add_test_data,
    get_bls12_381_complex_mul_test_data, get_bls12_381_complex_sub_test_data,
    get_bls12_381_curve_add_test_data, get_bls12_381_curve_dbl_test_data,
};

mod arith_eq_384_constants;
use arith_eq_384_constants::ARITH_EQ_384_ROWS_BY_OP;

// cargo run --release --bin arith_eq_384_test_generator

fn main() {
    let mut code = String::new();
    code += "#![no_main]\n";
    code += "#![cfg(all(target_os = \"zkvm\", target_vendor = \"zisk\"))]\n";
    code += "ziskos::entrypoint!(main);\n\n";
    code += "use ziskos::{\n";
    code += "\tcomplex::*, point::*, arith384_mod::*, bls12_381_curve_add::*, bls12_381_curve_dbl::*, bls12_381_complex_add::*, bls12_381_complex_sub::*, bls12_381_complex_mul::*,\n";
    code += "};\n\n";
    code += "fn main() {\n";
    code += "\tlet mut a:[u64;6] = [0,0,0,0,0,0];\n";
    code += "\tlet mut b:[u64;6] = [0,0,0,0,0,0];\n";
    code += "\tlet mut c:[u64;6] = [0,0,0,0,0,0];\n";
    code += "\tlet mut module:[u64;6] = [0,0,0,0,0,0];\n";
    code += "\tlet mut d:[u64;6] = [0,0,0,0,0,0];\n\n";
    code += "\tlet mut params = SyscallArith384ModParams {\n";
    code += "\t\ta: &mut a,\n";
    code += "\t\tb: &mut b,\n";
    code += "\t\tc: &mut c,\n";
    code += "\t\tmodule: &mut module,\n";
    code += "\t\td: &mut d,\n";
    code += "\t};\n\n";

    let mut index = 0;
    while let Some((a, b, c, module, d)) = get_arith384_mod_test_data(index) {
        code += &format!(
            "\t// arith384_mod test rows: {}-{}\n\n",
            index * ARITH_EQ_384_ROWS_BY_OP,
            (index + 1) * ARITH_EQ_384_ROWS_BY_OP - 1
        );
        code += &format!("\tparams.a = &{a:?};\n");
        code += &format!("\tparams.b = &{b:?};\n");
        code += &format!("\tparams.c = &{c:?};\n");
        code += &format!("\tparams.module = &{module:?};\n");
        code += "\tsyscall_arith384_mod(&mut params);\n";
        code += &format!("\tlet expected_d: [u64; 6] = {d:?};\n");
        code += "\tassert_eq!(params.d, &expected_d);\n\n";
        index += 1;
    }

    code += "\tlet mut p1 = SyscallPoint384 { x: [0,0,0,0,0,0], y: [0,0,0,0,0,0] };\n";
    code += "\tlet p2 = SyscallPoint384 { x: [0,0,0,0,0,0], y: [0,0,0,0,0,0] };\n";
    code += "\tlet mut params = SyscallBls12_381CurveAddParams { p1: &mut p1, p2: &p2 };\n";
    let initial_index = index;
    while let Some((p1, p2, p3)) = get_bls12_381_curve_add_test_data(index - initial_index) {
        code += &format!(
            "\t// bls12_381_curve_add test rows: {}-{}\n\n",
            index * ARITH_EQ_384_ROWS_BY_OP,
            (index + 1) * ARITH_EQ_384_ROWS_BY_OP - 1
        );
        let p1_x: [u64; 6] = p1[0..6].try_into().unwrap();
        let p1_y: [u64; 6] = p1[6..12].try_into().unwrap();
        code += &format!(
            "\tlet mut p1 = SyscallPoint384 {{\n\t\tx: {p1_x:?},\n\t\ty: {p1_y:?}\n\t}};\n"
        );
        let p2_x: [u64; 6] = p2[0..6].try_into().unwrap();
        let p2_y: [u64; 6] = p2[6..12].try_into().unwrap();
        code +=
            &format!("\tlet p2 = SyscallPoint384 {{\n\t\tx: {p2_x:?},\n\t\ty: {p2_y:?}\n\t}};\n");
        code += "\tparams.p1 = &mut p1;\n";
        code += "\tparams.p2 = &p2;\n";
        code += "\tsyscall_bls12_381_curve_add(&mut params);\n";

        let p3_x: [u64; 6] = p3[0..6].try_into().unwrap();
        let p3_y: [u64; 6] = p3[6..12].try_into().unwrap();
        code +=
            &format!("\tlet p3 = SyscallPoint384 {{\n\t\tx: {p3_x:?},\n\t\ty: {p3_y:?}\n\t}};\n");
        code += "\tassert_eq!(params.p1.x, p3.x);\n";
        code += "\tassert_eq!(params.p1.y, p3.y);\n\n";
        index += 1;
    }

    let initial_index = index;
    while let Some((p1, p3)) = get_bls12_381_curve_dbl_test_data(index - initial_index) {
        code += &format!(
            "\t// bls12_381_curve_dbl test rows: {}-{}\n\n",
            index * ARITH_EQ_384_ROWS_BY_OP,
            (index + 1) * ARITH_EQ_384_ROWS_BY_OP - 1
        );
        let p1_x: [u64; 6] = p1[0..6].try_into().unwrap();
        let p1_y: [u64; 6] = p1[6..12].try_into().unwrap();
        code += &format!(
            "\tlet mut p1 = SyscallPoint384 {{\n\t\tx: {p1_x:?},\n\t\ty: {p1_y:?}\n\t}};\n"
        );
        code += "\tsyscall_bls12_381_curve_dbl(&mut p1);\n";
        let p3_x: [u64; 6] = p3[0..6].try_into().unwrap();
        let p3_y: [u64; 6] = p3[6..12].try_into().unwrap();
        code +=
            &format!("\tlet p3 = SyscallPoint384 {{\n\t\tx: {p3_x:?},\n\t\ty: {p3_y:?}\n\t}};\n");
        code += "\tassert_eq!(&p1.x, &p3.x);\n";
        code += "\tassert_eq!(&p1.y, &p3.y);\n\n";
        index += 1;
    }

    code += "\tlet mut f1 = SyscallComplex384 { x: [0,0,0,0,0,0], y: [0,0,0,0,0,0] };\n";
    code += "\tlet f2 = SyscallComplex384 { x: [0,0,0,0,0,0], y: [0,0,0,0,0,0] };\n";
    code += "\tlet mut params = SyscallBls12_381ComplexAddParams { f1: &mut f1, f2: &f2 };\n";
    let initial_index = index;
    while let Some((f1, f2, f3)) = get_bls12_381_complex_add_test_data(index - initial_index) {
        code += &format!(
            "\t// bls12_381_complex_add test rows: {}-{}\n\n",
            index * ARITH_EQ_384_ROWS_BY_OP,
            (index + 1) * ARITH_EQ_384_ROWS_BY_OP - 1
        );
        let f1_x: [u64; 6] = f1[0..6].try_into().unwrap();
        let f1_y: [u64; 6] = f1[6..12].try_into().unwrap();
        code += &format!(
            "\tlet mut f1 = SyscallComplex384 {{\n\t\tx: {f1_x:?},\n\t\ty: {f1_y:?}\n\t}};\n"
        );
        let f2_x: [u64; 6] = f2[0..6].try_into().unwrap();
        let f2_y: [u64; 6] = f2[6..12].try_into().unwrap();
        code +=
            &format!("\tlet f2 = SyscallComplex384 {{\n\t\tx: {f2_x:?},\n\t\ty: {f2_y:?}\n\t}};\n");
        code += "\tparams.f1 = &mut f1;\n";
        code += "\tparams.f2 = &f2;\n";
        code += "\tsyscall_bls12_381_complex_add(&mut params);\n";

        let f3_x: [u64; 6] = f3[0..6].try_into().unwrap();
        let f3_y: [u64; 6] = f3[6..12].try_into().unwrap();
        code +=
            &format!("\tlet f3 = SyscallComplex384 {{\n\t\tx: {f3_x:?},\n\t\ty: {f3_y:?}\n\t}};\n");
        code += "\tassert_eq!(params.f1.x, f3.x);\n";
        code += "\tassert_eq!(params.f1.y, f3.y);\n\n";
        index += 1;
    }

    code += "\tlet mut params = SyscallBls12_381ComplexSubParams { f1: &mut f1, f2: &f2 };\n";
    let initial_index = index;
    while let Some((f1, f2, f3)) = get_bls12_381_complex_sub_test_data(index - initial_index) {
        code += &format!(
            "\t// bls12_381_complex_sub test rows: {}-{}\n\n",
            index * ARITH_EQ_384_ROWS_BY_OP,
            (index + 1) * ARITH_EQ_384_ROWS_BY_OP - 1
        );
        let f1_x: [u64; 6] = f1[0..6].try_into().unwrap();
        let f1_y: [u64; 6] = f1[6..12].try_into().unwrap();
        code += &format!(
            "\tlet mut f1 = SyscallComplex384 {{\n\t\tx: {f1_x:?},\n\t\ty: {f1_y:?}\n\t}};\n"
        );
        let f2_x: [u64; 6] = f2[0..6].try_into().unwrap();
        let f2_y: [u64; 6] = f2[6..12].try_into().unwrap();
        code +=
            &format!("\tlet f2 = SyscallComplex384 {{\n\t\tx: {f2_x:?},\n\t\ty: {f2_y:?}\n\t}};\n");
        code += "\tparams.f1 = &mut f1;\n";
        code += "\tparams.f2 = &f2;\n";
        code += "\tsyscall_bls12_381_complex_sub(&mut params);\n";

        let f3_x: [u64; 6] = f3[0..6].try_into().unwrap();
        let f3_y: [u64; 6] = f3[6..12].try_into().unwrap();
        code +=
            &format!("\tlet f3 = SyscallComplex384 {{\n\t\tx: {f3_x:?},\n\t\ty: {f3_y:?}\n\t}};\n");
        code += "\tassert_eq!(params.f1.x, f3.x);\n";
        code += "\tassert_eq!(params.f1.y, f3.y);\n\n";
        index += 1;
    }

    code += "\tlet mut params = SyscallBls12_381ComplexMulParams { f1: &mut f1, f2: &f2 };\n";
    let initial_index = index;
    while let Some((f1, f2, f3)) = get_bls12_381_complex_mul_test_data(index - initial_index) {
        code += &format!(
            "\t// bls12_381_complex_mul test rows: {}-{}\n\n",
            index * ARITH_EQ_384_ROWS_BY_OP,
            (index + 1) * ARITH_EQ_384_ROWS_BY_OP - 1
        );
        let f1_x: [u64; 6] = f1[0..6].try_into().unwrap();
        let f1_y: [u64; 6] = f1[6..12].try_into().unwrap();
        code += &format!(
            "\tlet mut f1 = SyscallComplex384 {{\n\t\tx: {f1_x:?},\n\t\ty: {f1_y:?}\n\t}};\n"
        );
        let f2_x: [u64; 6] = f2[0..6].try_into().unwrap();
        let f2_y: [u64; 6] = f2[6..12].try_into().unwrap();
        code +=
            &format!("\tlet f2 = SyscallComplex384 {{\n\t\tx: {f2_x:?},\n\t\ty: {f2_y:?}\n\t}};\n");
        code += "\tparams.f1 = &mut f1;\n";
        code += "\tparams.f2 = &f2;\n";
        code += "\tsyscall_bls12_381_complex_mul(&mut params);\n";

        let f3_x: [u64; 6] = f3[0..6].try_into().unwrap();
        let f3_y: [u64; 6] = f3[6..12].try_into().unwrap();
        code +=
            &format!("\tlet f3 = SyscallComplex384 {{\n\t\tx: {f3_x:?},\n\t\ty: {f3_y:?}\n\t}};\n");
        code += "\tassert_eq!(params.f1.x, f3.x);\n";
        code += "\tassert_eq!(params.f1.y, f3.y);\n\n";
        index += 1;
    }

    code += "}\n\n";
    code = rustfmt_wrapper::rustfmt(code).unwrap();
    println!("{code}");
}
