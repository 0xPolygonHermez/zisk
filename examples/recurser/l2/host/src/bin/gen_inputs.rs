//! Writes the three segment inputs to `a.bin` / `b.bin` / `c.bin` so the CLI
//! walk in the README can feed them with `prove -i a.bin`. The leaf input is the
//! 256-byte ABI encoding of a `BlocksInfoStruct`, wrapped as one ZiskStdin frame
//! — not something you'd hand-type, so this dumps the exact bytes.
//!
//! Run: `cargo run --release -p recurser-l2-host --bin gen-inputs-l2`

use std::error::Error;

use alloy_sol_types::SolValue;
use recurser_l2_common::segment;
use zisk_sdk::ZiskStdin;

fn main() -> Result<(), Box<dyn Error>> {
    let segments = [segment(100, 200), segment(200, 300), segment(300, 400)];
    for (seg, name) in segments.iter().zip(["a.bin", "b.bin", "c.bin"]) {
        let stdin = ZiskStdin::new();
        stdin.write_slice(&seg.abi_encode()); // matches prove_segment in main.rs
        stdin.save(std::path::Path::new(name))?;
        println!("wrote {name}");
    }
    Ok(())
}
