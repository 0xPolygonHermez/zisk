#![no_main]
#![allow(unexpected_cfgs)]
ziskos::entrypoint!(main);

#[cfg(feature = "software")]
#[path = "../../../../precompiles/helpers/src/fp32.rs"]
mod fp32;

fn dot(x: &[u64], y: &[u64], n: usize, initial: u32, flags: u8) -> u64 {
    #[cfg(feature = "software")]
    {
        let unpack = |words: &[u64]| -> Vec<u32> {
            words.iter().flat_map(|&w| [w as u32, (w >> 32) as u32]).take(n).collect()
        };
        let result =
            fp32::fp32_dot(&unpack(x), &unpack(y), fp32::Fp32Result { bits: initial, flags })
                .unwrap();
        result.bits as u64 | ((result.flags as u64) << 32)
    }
    #[cfg(not(feature = "software"))]
    {
        ziskos::syscalls::syscall_fp32_dot(x, y, n, initial, flags).unwrap()
    }
}

fn main() {
    // Black-boxed vectors prevent constant folding in the software comparison.
    let cases = core::hint::black_box([
        (
            [0x3f800000_4b800000, 0x7f800001_cb800000],
            [0x3f800000_3f800000, 0x7f800001_3f800000],
            3usize,
            1u64 << 32,
        ),
        ([0x00010000_00010000, 0], [0x37000000_37800000, 0], 2, (3u64 << 32) | 1),
        // Three-tap FIR: 1/2 - 2/4 - 3/8 = -3/8.
        ([0xc0000000_3f800000, 0x40400000], [0x3e800000_3f000000, 0xbe000000], 3, 0xbec00000),
    ]);
    let mut output = Vec::new();
    for (x, y, n, expected) in cases {
        let result = dot(&x[..n.div_ceil(2)], &y[..n.div_ceil(2)], n, 0, 0);
        assert_eq!(result, expected);
        output.extend_from_slice(&result.to_le_bytes());
    }
    let ones = core::hint::black_box(vec![0x3f800000_3f800000; 2048]);
    let result = dot(&ones, &ones, 4096, 0, 0);
    assert_eq!(result, 0x45800000);
    output.extend_from_slice(&result.to_le_bytes());
    let empty = dot(&[], &[], 0, core::hint::black_box(0xff800001), 8);
    assert_eq!(empty, (8u64 << 32) | 0xff800001);
    output.extend_from_slice(&empty.to_le_bytes());
    ziskos::io::commit_slice(&output);
}
