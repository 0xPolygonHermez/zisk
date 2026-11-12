//! zisk-patch-hashes / sha3: Keccak-f[1600] routed to `syscall_keccak_f`.

#![no_main]
ziskos::entrypoint!(main);

use patch_test_common::{Rng, Suite};
use sha3::{
    digest::{ExtendableOutput, Update, XofReader},
    Digest, Keccak256, Keccak512, Sha3_256, Sha3_512, Shake128,
};

fn main() {
    let mut t = Suite::new("sha3");

    t.eq_hex("keccak256('')", &Keccak256::digest(b""), "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");
    t.eq_hex("keccak256('abc')", &Keccak256::digest(b"abc"), "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45");
    t.eq_hex(
        "keccak512('')",
        &Keccak512::digest(b""),
        "0eab42de4c3ceb9235fc91acffe746b29c29a8c366b7c60e4e67c466f36a4304c00fa9caf9d87976ba469bcbe06713b435f091ef2769fb160cdab33d3670680e",
    );
    t.eq_hex("sha3-256('')", &Sha3_256::digest(b""), "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a");
    t.eq_hex("sha3-256('abc')", &Sha3_256::digest(b"abc"), "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532");
    t.eq_hex(
        "sha3-512('abc')",
        &Sha3_512::digest(b"abc"),
        "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0",
    );
    t.eq_hex(
        "sha3-256(1M x 'a')",
        &Sha3_256::digest(vec![b'a'; 1_000_000]),
        "5c8875ae474a3634ba4fd55ec85bffd661f32aca75c6d699d0cdcb6c115891c1",
    );

    let mut xof = Shake128::default();
    xof.update(b"");
    let mut out = [0u8; 32];
    xof.finalize_xof().read(&mut out);
    t.eq_hex("shake128('', 32)", &out, "7f9c2ba4e88f827d616045507605853ed73b8093f6efbc88eb1a6eacfa66ef26");

    // Rate-boundary lengths (keccak256 rate = 136 bytes), split incrementally.
    let mut rng = Rng::new(0x5a3);
    for len in [135usize, 136, 137, 271, 272, 273, 1000, 4097] {
        let msg = rng.bytes(len);
        let one_shot = Keccak256::digest(&msg);
        let mut h = Keccak256::new();
        for chunk in msg.chunks(11) {
            Digest::update(&mut h, chunk);
        }
        t.eq(&format!("keccak256 incremental == one-shot (len {len})"), h.finalize(), one_shot);
    }

    t.finish();
}
