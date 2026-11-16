use zisk_precomp_helpers::{fp32_add, fp32_dot, fp32_mul, Fp32Result, FLOATDOT_MAX_LEN};

fn r(bits: u32, flags: u8) -> Fp32Result {
    Fp32Result { bits, flags }
}

#[test]
fn add_rounds_ties_cancellation_and_signed_zeros() {
    for (a, b, bits, flags) in [
        (0x3f800000, 0x33800000, 0x3f800000, 1),
        (0x3f800001, 0x33800000, 0x3f800002, 1),
        (0x3f800000, 0xbf800000, 0, 0),
        (0x80000000, 0x80000000, 0x80000000, 0),
        (0, 0x80000000, 0, 0),
        (0x00800000, 0x807fffff, 1, 0),
        (0x007fffff, 1, 0x00800000, 0),
        (0x7f7fffff, 0x7f7fffff, 0x7f800000, 5),
        (0x3f800000, 0x80000001, 0x3f800000, 1),
    ] {
        assert_eq!(fp32_add(a, b), r(bits, flags), "{a:08x}+{b:08x}");
        assert_eq!(fp32_add(b, a), r(bits, flags));
    }
}

#[test]
fn multiply_rounds_subnormals_and_overflow() {
    for (a, b, bits, flags) in [
        (0x3fc00000, 0x40000000, 0x40400000, 0),
        (1, 0x3f000000, 0, 3),
        (3, 0x3f000000, 2, 3),
        (0x80000001, 0x3f000000, 0x80000000, 3),
        (0x00800000, 0x3f000000, 0x00400000, 0),
        (0x00800000, 0x3f7fffff, 0x00800000, 3),
        (0x7f7fffff, 0x40000000, 0x7f800000, 5),
        (0x80000000, 0xbf800000, 0, 0),
        (0, 0xbf800000, 0x80000000, 0),
        (0x3f800001, 0x3fc00000, 0x3fc00002, 1),
    ] {
        assert_eq!(fp32_mul(a, b), r(bits, flags), "{a:08x}*{b:08x}");
        assert_eq!(fp32_mul(b, a), r(bits, flags));
    }
}

#[test]
fn exceptional_values_are_canonical_and_set_only_required_flags() {
    for (a, b, add, mul) in [
        (0x7f800000, 0xff800000, r(0x7fc00000, 16), r(0xff800000, 0)),
        (0, 0xff800000, r(0xff800000, 0), r(0x7fc00000, 16)),
        (0xffc01234, 0x3f800000, r(0x7fc00000, 0), r(0x7fc00000, 0)),
        (0x7f800001, 0x3f800000, r(0x7fc00000, 16), r(0x7fc00000, 16)),
        (0xffc01234, 0x7f800001, r(0x7fc00000, 16), r(0x7fc00000, 16)),
        (0x7f800000, 0x7f800000, r(0x7f800000, 0), r(0x7f800000, 0)),
    ] {
        assert_eq!(fp32_add(a, b), add);
        assert_eq!(fp32_mul(a, b), mul);
    }
}

#[test]
fn dot_preserves_order_two_roundings_and_chunk_state() {
    let x = [0x4b800000, 0x3f800000, 0xcb800000];
    let y = [0x3f800000; 3];
    assert_eq!(fp32_dot(&x, &y, r(0, 0)).unwrap(), r(0, 1));
    // BF16 a=2^-133, products 2^-149 and 2^-150; FMA would return bits 2.
    let x = [0x00010000; 2];
    let y = [0x37800000, 0x37000000];
    assert_eq!(fp32_dot(&x, &y, r(0, 0)).unwrap(), r(1, 3));
    for split in 0..=2 {
        let first = fp32_dot(&x[..split], &y[..split], r(0, 8)).unwrap();
        assert_eq!(fp32_dot(&x[split..], &y[split..], first).unwrap(), r(1, 11));
    }
    for flags in 0..32 {
        assert_eq!(fp32_dot(&[], &[], r(0xff800001, flags)).unwrap(), r(0xff800001, flags));
    }
    // A three-tap FIR: [1,-2,3] dot [1/2,1/4,-1/8] = -3/8.
    assert_eq!(
        fp32_dot(
            &[0x3f800000, 0xc0000000, 0x40400000],
            &[0x3f000000, 0x3e800000, 0xbe000000],
            r(0, 0)
        )
        .unwrap(),
        r(0xbec00000, 0)
    );
}

#[test]
fn dot_rejects_lengths_and_reserved_flags_before_work() {
    assert!(fp32_dot(&[0], &[], r(0, 0)).is_err());
    assert!(fp32_dot(&[], &[], r(0, 32)).is_err());
    let x = vec![0x3f800000; FLOATDOT_MAX_LEN + 1];
    assert!(fp32_dot(&x, &x, r(0, 0)).is_err());
    assert_eq!(
        fp32_dot(&x[..FLOATDOT_MAX_LEN], &x[..FLOATDOT_MAX_LEN], r(0, 0)).unwrap(),
        r((FLOATDOT_MAX_LEN as f32).to_bits(), 0)
    );
}

#[test]
#[ignore = "run node precompiles/helpers/tests/fp32_oracle.cjs to build the bundled SoftFloat oracle"]
fn differential_softfloat_bits_and_flags() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let executable = std::env::var_os("FP32_SOFTFLOAT_ORACLE").expect("oracle executable");
    let edge = [
        0, 1, 2, 3, 0x007ffffe, 0x007fffff, 0x00800000, 0x00800001, 0x3effffff, 0x3f000000,
        0x3f000001, 0x3f7fffff, 0x3f800000, 0x3f800001, 0x7f7fffff, 0x7f800000, 0x7f800001,
        0x7fc00000, 0x7fffffff,
    ];
    let mut pairs = Vec::new();
    for a in edge {
        for b in edge {
            for signs in 0..4 {
                pairs.push((a | ((signs & 1) << 31), b | ((signs >> 1) << 31)));
            }
        }
    }
    let mut seed = 0x4d595df4d0f33173u64;
    for _ in 0..100_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        pairs.push((seed as u32, (seed >> 32) as u32));
    }
    let mut child =
        Command::new(executable).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let queries = pairs.clone();
    let writer = std::thread::spawn(move || {
        for (a, b) in queries {
            writeln!(input, "{a:08x} {b:08x}").unwrap();
        }
    });
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = output.lines().collect();
    assert_eq!(lines.len(), pairs.len());
    for ((a, b), line) in pairs.into_iter().zip(lines) {
        let values: Vec<_> =
            line.split_whitespace().map(|s| u32::from_str_radix(s, 16).unwrap()).collect();
        assert_eq!(values.len(), 4);
        assert_eq!(fp32_add(a, b), r(values[0], values[1] as u8), "add {a:08x} {b:08x}");
        assert_eq!(fp32_mul(a, b), r(values[2], values[3] as u8), "mul {a:08x} {b:08x}");
    }
}
