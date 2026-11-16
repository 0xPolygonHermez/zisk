use zisk_precomp_helpers::{Fp32Witness, FP32_WITNESS_COLUMNS};

fn pack(bits: &[u64]) -> u64 {
    bits.iter().enumerate().map(|(i, bit)| bit << i).sum()
}

#[test]
fn scalar_witness_preserves_exceptional_results_and_rounding_flags() {
    for (a, b, is_mul, result, flags) in [
        (0x3f800000, 0x33800000, false, 0x3f800000, 1),
        (0x3f800001, 0x33800000, false, 0x3f800002, 1),
        (0x80000000, 0x80000000, false, 0x80000000, 0),
        (0x3f800000, 0xbf800000, false, 0, 0),
        (0x00800000, 0x3f7fffff, true, 0x00800000, 3),
        (0x80000001, 0x3f000000, true, 0x80000000, 3),
        (0x7f7fffff, 0x40000000, true, 0x7f800000, 5),
        (0x7f800000, 0xff800000, false, 0x7fc00000, 16),
        (0x7f800001, 0x7fc00000, true, 0x7fc00000, 16),
        (0x7fc00000, 0x3f800000, false, 0x7fc00000, 0),
    ] {
        let witness = Fp32Witness::new(a, b, is_mul);
        assert_eq!(pack(&witness.result), result, "{a:08x} {b:08x} mul={is_mul}");
        assert_eq!(pack(&witness.flags), flags);
    }
}

#[test]
fn zero_and_nonfinite_inputs_fill_the_inactive_numeric_path() {
    let witness = Fp32Witness::new(0, 0, true);
    assert_eq!(pack(&witness.fp_norm_high_index), 0);
    assert_eq!(witness.fp_norm_prefix, [1; 49]);
    assert_eq!(pack(&witness.fp_exponent), 214); // -298 biased by 512
    assert_eq!(pack(&witness.fp_round_shift_amount), 172);
    assert_eq!(pack(&witness.fp_round_shift_bits), 63);
    assert_eq!(pack(&witness.fp_norm_sig), 0);

    let witness = Fp32Witness::new(0x7f800000, 0x80000000, true);
    assert_eq!(pack(&witness.fp_a_sig), 0x800000);
    assert_eq!(pack(&witness.fp_a_finite_sig), 0);
    assert_eq!(witness.fp_a_eff_exp, 1);
    assert_eq!(pack(&witness.fp_round_result), 0x80000000);
    assert_eq!((witness.fp_invalid, witness.fp_nan, witness.fp_inf), (1, 1, 0));

    let witness = Fp32Witness::new(0xff800000, 0x3f800000, false);
    assert_eq!(pack(&witness.fp_norm_sig), 1 << 26);
    assert_eq!(pack(&witness.fp_round_result), 0x3f800000);
    assert_eq!(pack(&witness.result), 0xff800000);
}

#[test]
fn flat_row_uses_the_scalar_pil_column_order() {
    let witness = Fp32Witness::new(0x3f800001, 0x3f7fffff, true);
    let row = witness.to_row();
    assert_eq!(row.len(), FP32_WITNESS_COLUMNS);
    assert_eq!(row.len(), 1813);
    assert_eq!(pack(&row[0..32]), 0x3f800001);
    assert_eq!(pack(&row[32..64]), 0x3f7fffff);
    assert_eq!(row[64], 1);
    assert_eq!(pack(&row[65..97]), 0x3f800000);
    assert_eq!(pack(&row[97..102]), 1);
    assert_eq!(&row[102..126], &witness.fp_a_sig);
    assert_eq!(&row[176..200], &witness.fp_b_sig);
    assert_eq!(&row[346..634], witness.fp_align_stages.as_flattened());
    assert_eq!(&row[634..832], witness.fp_align_sticky.as_flattened());
    assert_eq!(&row[1773..1805], &witness.fp_round_result);
    assert_eq!(&row[1810..1813], &[0, 0, 0]);
}

#[test]
#[ignore = "writes CSV witnesses for compiled PIL and independent SoftFloat evaluation"]
fn dump_scalar_witness_rows() {
    use std::io::Write;
    let path = std::env::var_os("FP32_WITNESS_DUMP").expect("set FP32_WITNESS_DUMP");
    let mut output = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let edges = [
        0, 1, 3, 0x007fffff, 0x00800000, 0x3f000000, 0x3f7fffff, 0x3f800000, 0x3f800001,
        0x7f7fffff, 0x7f800000, 0x7f800001, 0x7fc00000,
    ];
    let mut pairs = Vec::new();
    for a in edges {
        for b in edges {
            for signs in 0..4 {
                pairs.push((a | ((signs & 1) << 31), b | ((signs >> 1) << 31)));
            }
        }
    }
    let mut seed = 0x4d595df4d0f33173u64;
    for _ in 0..1000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        pairs.push((seed as u32, (seed >> 32) as u32));
    }
    for (a, b) in pairs {
        for is_mul in [false, true] {
            for (index, value) in Fp32Witness::new(a, b, is_mul).to_row().iter().enumerate() {
                if index != 0 {
                    write!(output, ",").unwrap();
                }
                write!(output, "{value}").unwrap();
            }
            writeln!(output).unwrap();
        }
    }
    output.flush().unwrap();
}
