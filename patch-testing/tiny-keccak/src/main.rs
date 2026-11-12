//! zisk-patch-tiny-keccak: Keccak-f[1600] routed to `syscall_keccak_f`.

#![no_main]
ziskos::entrypoint!(main);

use patch_test_common::{Rng, Suite};
use tiny_keccak::{Hasher, Keccak, Sha3};

fn keccak256(chunks: &[&[u8]]) -> [u8; 32] {
    let mut h = Keccak::v256();
    for c in chunks {
        h.update(c);
    }
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

fn main() {
    let mut t = Suite::new("tiny-keccak");

    t.eq_hex("keccak256('')", &keccak256(&[b""]), "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");
    t.eq_hex("keccak256('abc')", &keccak256(&[b"abc"]), "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45");
    t.eq_hex(
        "keccak256(1M x 'a')",
        &keccak256(&[&vec![b'a'; 1_000_000]]),
        "fadae6b49f129bbb812be8407b7b2894f34aecf6dbd1f9b0f0c7e9853098fc96",
    );

    let mut sha3 = Sha3::v256();
    sha3.update(b"abc");
    let mut out = [0u8; 32];
    sha3.finalize(&mut out);
    t.eq_hex("sha3-256('abc')", &out, "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532");

    let mut rng = Rng::new(0x7c);
    for len in [135usize, 136, 137, 271, 272, 273, 1000, 4097] {
        let msg = rng.bytes(len);
        let parts: Vec<&[u8]> = msg.chunks(13).collect();
        t.eq(&format!("keccak256 incremental == one-shot (len {len})"), keccak256(&parts), keccak256(&[&msg]));
    }

    t.finish();
}
