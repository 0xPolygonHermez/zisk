//! Writes the three leaf inputs to `a.bin` / `b.bin` / `c.bin` so the CLI walk
//! in the README can feed them with `prove -i a.bin`. The leaf guest reads a
//! `[u64; 12]` via `ziskos::io::read` (bincode), so a hand-typed `inline://`
//! wouldn't match — this dumps the exact bytes the host would write.
//!
//! Run: `cargo run --release -p recurser-hash-host --bin gen-inputs-hash`

use std::error::Error;

use recurser_hash_common::secret_vectors;
use zisk_sdk::ZiskStdin;

fn main() -> Result<(), Box<dyn Error>> {
    for (secret, name) in secret_vectors().iter().zip(["a.bin", "b.bin", "c.bin"]) {
        let stdin = ZiskStdin::new();
        stdin.write(secret); // one [u64; 12] frame, matching the guest's read
        stdin.save(std::path::Path::new(name))?;
        println!("wrote {name}");
    }
    Ok(())
}
