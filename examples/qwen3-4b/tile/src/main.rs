#![no_main]

#[path = "../../guest/src/bf16_projection.rs"]
mod bf16_projection;

use ziskos::syscalls::{DOT_F8_E4M3, SyscallDotF8Params, dot_f8_formats, syscall_dot_f8};

ziskos::entrypoint!(main);

const POSITIONS: usize = 39;
const ROWS: usize = 64;
const WIDTH: usize = 2560;

struct LosslessDot;

impl pci_dotf8::DotF8 for LosslessDot {
    #[inline(always)]
    fn block_sums(&mut self, x: &[u64], y: &[u64], sums: &mut [i64]) {
        assert_eq!(x.len(), sums.len());
        assert_eq!(y.len(), sums.len());
        let params = SyscallDotF8Params {
            x: x.as_ptr().cast(),
            y: y.as_ptr().cast(),
            n: 8 * sums.len() as u64,
            formats: dot_f8_formats(DOT_F8_E4M3, DOT_F8_E4M3),
            block: 8,
            out: sums.as_mut_ptr(),
        };
        // Each aligned input word supplies one block and sums has one slot per block.
        unsafe { syscall_dot_f8(&params) };
    }
}

fn main() {
    let input = ziskos::io::read_slice();
    assert_eq!(input.len(), 24 + 4 * POSITIONS * WIDTH + 2 * ROWS * WIDTH);
    assert_eq!(&input[..8], b"PCISMPL1");
    let word = |i| u32::from_le_bytes(input[i..i + 4].try_into().unwrap()) as usize;
    assert_eq!((word(12), word(16), word(20)), (POSITIONS, ROWS, WIDTH));
    let (x_bytes, weights) = input[24..].split_at(4 * POSITIONS * WIDTH);
    // The input frame and its header preserve u32 alignment on the little-endian guest.
    let (prefix, x, suffix) = unsafe { x_bytes.align_to::<u32>() };
    assert!(prefix.is_empty() && suffix.is_empty());
    let mut output = vec![0f32; POSITIONS * ROWS];
    match word(8) {
        5 => pci_dotf8::Kernel::new(LosslessDot).project_group_bf16(
            x,
            WIDTH,
            &[weights],
            &mut [&mut output],
        ),
        6 => bf16_projection::project(x, WIDTH, &[weights], &mut [&mut output]),
        _ => panic!("expected mode 5 or 6"),
    }
    let bytes: Vec<u8> = output.iter().flat_map(|v| v.to_le_bytes()).collect();
    ziskos::io::commit_slice(&ziskos::zisklib::sha256(&bytes));
}
