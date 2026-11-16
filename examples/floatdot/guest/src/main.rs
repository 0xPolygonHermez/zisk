#![no_main]
#![allow(unexpected_cfgs)]
ziskos::entrypoint!(main);

#[cfg(feature = "software")]
#[path = "../../../../precompiles/helpers/src/fp32.rs"]
mod fp32;

#[derive(Clone, Copy)]
enum Format {
    Bf16,
    Fp16,
}

#[allow(clippy::too_many_arguments)]
fn dot(
    f: Format,
    x: &[u16],
    sx: usize,
    y: &[u16],
    sy: usize,
    n: usize,
    initial: u32,
    flags: u8,
) -> u64 {
    #[cfg(feature = "software")]
    {
        let widen = |v: &[u16], s: usize| -> Vec<u32> {
            (0..n)
                .map(|i| match f {
                    Format::Bf16 => fp32::bf16_to_fp32(v[i * s]),
                    Format::Fp16 => fp32::fp16_to_fp32(v[i * s]),
                })
                .collect()
        };
        let initial = fp32::Fp32Result { bits: initial, flags };
        let result = fp32::fp32_dot(&widen(x, sx), &widen(y, sy), initial).unwrap();
        result.bits as u64 | ((result.flags as u64) << 32)
    }
    #[cfg(not(feature = "software"))]
    match f {
        Format::Bf16 => {
            ziskos::syscalls::syscall_floatdot_bf16(x, sx, y, sy, n, initial, flags).unwrap()
        }
        Format::Fp16 => {
            ziskos::syscalls::syscall_floatdot_fp16(x, sx, y, sy, n, initial, flags).unwrap()
        }
    }
}

fn dot32(x: &[u64], y: &[u64], n: usize) -> u64 {
    #[cfg(feature = "software")]
    {
        let unpack = |w: &[u64]| -> Vec<u32> {
            w.iter().flat_map(|&w| [w as u32, (w >> 32) as u32]).take(n).collect()
        };
        let r = fp32::fp32_dot(&unpack(x), &unpack(y), fp32::Fp32Result { bits: 0, flags: 0 });
        let r = r.unwrap();
        r.bits as u64 | ((r.flags as u64) << 32)
    }
    #[cfg(not(feature = "software"))]
    {
        ziskos::syscalls::syscall_fp32_dot(x, y, n, 0, 0).unwrap()
    }
}

fn main() {
    use Format::*;
    let mut seed = core::hint::black_box(0x9e37_79b9_7f4a_7c15u64);
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    // Finite values of both signs over a few binades.
    let bf = |r: u64| ((r & 0x8000) | ((119 + (r >> 16) % 16) << 7) | ((r >> 32) & 0x7f)) as u16;
    let fp = |r: u64| ((r & 0x8000) | ((5 + (r >> 16) % 20) << 10) | ((r >> 32) & 0x3ff)) as u16;
    let mut out = Vec::new();

    // Q·K-like: contiguous BF16 vectors.
    let q: Vec<u16> = (0..128).map(|_| bf(next())).collect();
    let k: Vec<u16> = (0..128).map(|_| bf(next())).collect();
    out.push(dot(Bf16, &q, 1, &k, 1, 128, 0, 0));

    // P·V-like: contiguous probabilities, one V column at stride 1024 with an odd length.
    // The other lanes of every enclosing word hold a signaling-NaN pattern and are ignored.
    let mut words = vec![0x7fa1_7fa1_7fa1_7fa1u64; 33 * 256];
    // SAFETY: u64 storage is suitably aligned and sized for four u16 lanes per word.
    let v = unsafe { core::slice::from_raw_parts_mut(words.as_mut_ptr() as *mut u16, 33 * 1024) };
    for t in 0..33 {
        v[t * 1024 + 5] = bf(next());
    }
    let p: Vec<u16> = (0..33).map(|_| bf(next()) & 0x7fff).collect();
    out.push(dot(Bf16, &p, 1, &v[5..], 1024, 33, 0, 0));

    // FP16 exceptional values at stride 3 against a broadcast 1.0: subnormals, -0, max,
    // infinities and a signaling NaN (NV), continuing from 1.0 with initial DZ.
    let x16 = core::hint::black_box([
        0x0001u16, 0, 0, 0x03ff, 0, 0, 0x8000, 0, 0, 0x7bff, 0, 0, 0x7c00, 0, 0, 0xfc00, 0, 0,
        0x7d00,
    ]);
    out.push(dot(Fp16, &x16, 3, &[0x3c00], 0, 7, 0x3f80_0000, 8));
    let a: Vec<u16> = (0..64).map(|_| fp(next())).collect();
    let b: Vec<u16> = (0..64).map(|_| fp(next())).collect();
    out.push(dot(Fp16, &a, 1, &b, 1, 64, 0, 0));

    // Continuation: 4099 BF16 terms as a maximum-length call plus a 3-term call.
    let x: Vec<u16> = (0..4099).map(|_| bf(next())).collect();
    let y: Vec<u16> = (0..4099).map(|_| bf(next())).collect();
    let head = dot(Bf16, &x, 1, &y, 1, 4096, 0, 0);
    out.push(dot(Bf16, &x[4096..], 1, &y[4096..], 1, 3, head as u32, (head >> 32) as u8));

    // Empty call keeps a negative signaling NaN and flags verbatim.
    out.push(dot(Bf16, &[], 1, &[], 1, 0, core::hint::black_box(0xff80_0001), 24));

    // The legacy FP32 path shares the AIR: ordered odd tail [2^24, 1, -2^24].
    let x32 = core::hint::black_box([0x3f800000_4b800000, 0x7f800001_cb800000]);
    let y32 = core::hint::black_box([0x3f800000_3f800000, 0x7f800001_3f800000]);
    out.push(dot32(&x32, &y32, 3));

    let bytes: Vec<u8> = out.iter().flat_map(|w| w.to_le_bytes()).collect();
    ziskos::io::commit_slice(&bytes);
}
